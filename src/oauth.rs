use http::Uri;
use serde::{Deserialize, Serialize};

use crate::newtypes::UserId;

#[derive(Clone, Debug, Serialize)]
pub struct OauthExchangeRequest {
    pub client_id: String,
    pub client_secret: String,
    pub code: String,
    #[serde(with = "http_serde::option::uri")]
    pub redirect_uri: Option<Uri>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OauthExchangeResponse {
    pub access_token: String,
    pub app_id: String,
    pub authed_user: AuthedUser,
    pub bot_user_id: UserId,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthedUser {
    pub id: UserId,
}
