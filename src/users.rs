use email_address::EmailAddress;
use serde::{Deserialize, Serialize};

use crate::newtypes::UserId;

#[derive(Serialize)]
pub struct GetUserInfoRequest {
    pub user: UserId,
}

#[derive(Debug, Deserialize)]
pub struct GetUserInfoResponse {
    pub user: UserInfo,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserInfo {
    pub id: UserId,
    pub team_id: String,
    pub name: String,
    pub real_name: String,
    pub profile: UserProfile,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserProfile {
    pub real_name: String,
    pub display_name: String,
    pub email: Option<EmailAddress>,
}
