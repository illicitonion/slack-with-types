use serde::Deserialize;

pub mod client;
pub mod newtypes;
pub mod oauth;
pub mod usergroups;
pub mod users;

#[derive(Deserialize)]
pub struct Response<T> {
    pub ok: bool,
    #[serde(flatten)]
    pub response: Option<T>,
}

impl<T> Response<T> {
    pub fn into_result(self) -> Result<T, Error> {
        let Response { ok, response } = self;
        if ok {
            match response {
                Some(response) => Ok(response),
                None => Err(Error::OkButNoResponse),
            }
        } else {
            Err(Error::Unknown)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Exhausted rates limits")]
    ExhaustedRateLimits,
    #[error("HTTP error: {0:?}")]
    Http(reqwest::Error),
    #[error("Response was ok but had no body")]
    OkButNoResponse,
    #[error("Failed to encode request body: {0:?}")]
    RequestEncoding(serde_urlencoded::ser::Error),
    #[error("Failed to decode response body: {0:?}")]
    ResponseDecoding(serde_json::Error),
    #[error("unknown")]
    Unknown,
}
