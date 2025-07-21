use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{FutureExt, future::Shared};
use governor::{
    Jitter, Quota,
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
};
use serde::{Serialize, de::DeserializeOwned};
use tokio::time::{Instant, Sleep};
use tracing::{error, warn};

use crate::Error;

pub struct Client {
    client: reqwest::Client,
    auth_token: Option<String>,
    rate_limiter: RateLimiter,
}

impl Client {
    pub fn new(client: reqwest::Client, rate_limiter: RateLimiter, auth_token: String) -> Client {
        Client::new_internal(client, rate_limiter, Some(auth_token))
    }

    pub fn new_without_auth(client: reqwest::Client, rate_limiter: RateLimiter) -> Client {
        Client::new_internal(client, rate_limiter, None)
    }

    fn new_internal(
        client: reqwest::Client,
        rate_limiter: RateLimiter,
        auth_token: Option<String>,
    ) -> Client {
        Client {
            client,
            auth_token,
            rate_limiter,
        }
    }

    pub async fn post<Body: Serialize, Response: DeserializeOwned>(
        &self,
        endpoint: &str,
        body: Body,
    ) -> Result<Response, Error> {
        let body = serde_urlencoded::to_string(&body).map_err(|err| Error::RequestEncoding(err))?;

        let mut attempt = 0_usize;
        loop {
            attempt += 1;
            if attempt > 5 {
                return Err(Error::ExhaustedRateLimits);
            }
            if let Some(rate_limiter) = self.rate_limiter.get(endpoint) {
                rate_limiter.until_ready().await;
            }
            let mut request = self
                .client
                .post(format!("https://slack.com/api/{endpoint}"))
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(reqwest::header::ACCEPT, "application/json");
            if let Some(auth_token) = &self.auth_token {
                request = request.header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", auth_token),
                );
            }
            let response = request
                .body(body.clone())
                .send()
                .await
                .map_err(|err| Error::Http(err))?;
            if response.status().as_u16() == 429 {
                if let Some(retry_after_header) =
                    response.headers().get(reqwest::header::RETRY_AFTER)
                {
                    if let Ok(retry_after_header_str) = retry_after_header.to_str() {
                        if let Ok(seconds) = retry_after_header_str.parse::<u8>() {
                            let retry_after = Duration::from_secs(u64::from(seconds));
                            warn!(%endpoint, ?retry_after, "Encountered rate limiting");
                            if let Some(rate_limiter) = self.rate_limiter.get(endpoint) {
                                rate_limiter.set_retry_after(retry_after);
                                continue;
                            }
                        }
                    }
                }
                error!(
                    "Encountered rate limiting but couldn't interpret retry instructions - header: {:?}",
                    response.headers().get(reqwest::header::RETRY_AFTER)
                );
            }
            let response: crate::Response<Response> = response
                .error_for_status()
                .map_err(|err| Error::Http(err))?
                .json()
                .await
                .map_err(|err| Error::Http(err))?;

            return response.into_result();
        }
    }
}

// See https://api.slack.com/apis/rate-limits

#[derive(Clone)]
pub struct RateLimiter(Arc<HashMap<&'static str, RateLimiterWithRetryAfterHandling>>);

impl RateLimiter {
    pub fn new() -> RateLimiter {
        let mut rate_limiters = HashMap::new();
        rate_limiters.insert(
            "usergroups.list",
            RateLimiterWithRetryAfterHandling::new(RateLimitingTier::Tier2),
        );
        rate_limiters.insert(
            "usergroups.users.list",
            RateLimiterWithRetryAfterHandling::new(RateLimitingTier::Tier2),
        );
        rate_limiters.insert(
            "users.info",
            RateLimiterWithRetryAfterHandling::new(RateLimitingTier::Tier4),
        );
        RateLimiter(Arc::new(rate_limiters))
    }

    fn get(&self, endpoint: &str) -> Option<&RateLimiterWithRetryAfterHandling> {
        self.0.get(endpoint)
    }
}

#[derive(Clone)]
struct RateLimiterWithRetryAfterHandling {
    rate_limiter: Arc<governor::RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    retry_after_future: Arc<Mutex<Option<Shared<Sleep>>>>,
}

impl RateLimiterWithRetryAfterHandling {
    fn new(tier: RateLimitingTier) -> Self {
        Self {
            rate_limiter: Arc::new(governor::RateLimiter::direct(tier.quota())),
            retry_after_future: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn until_ready(&self) -> () {
        self.rate_limiter
            .until_ready_with_jitter(Jitter::up_to(Duration::from_millis(200)))
            .await;
        let retry_after_future =
            if let Some(retry_after_future) = self.retry_after_future.lock().unwrap().as_ref() {
                Some(retry_after_future.clone())
            } else {
                None
            };
        if let Some(retry_after_future) = retry_after_future {
            retry_after_future.await;
        }
    }

    fn set_retry_after(&self, duration: Duration) {
        let mut retry_after_future = self.retry_after_future.lock().unwrap();
        // TODO: Pick last time, rather than first reported time.
        if retry_after_future.is_some() {
            return;
        }
        *retry_after_future = Some(tokio::time::sleep_until(Instant::now() + duration).shared());
    }
}

#[derive(Clone, Copy)]
enum RateLimitingTier {
    #[allow(unused)]
    Tier1,
    Tier2,
    #[allow(unused)]
    Tier3,
    Tier4,
}

impl RateLimitingTier {
    fn quota(&self) -> Quota {
        Quota::per_minute(
            NonZeroU32::new(match self {
                Self::Tier1 => 1,
                Self::Tier2 => 20,
                Self::Tier3 => 50,
                Self::Tier4 => 100,
            })
            .unwrap(),
        )
    }
}
