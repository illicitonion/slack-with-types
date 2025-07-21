use serde::{Deserialize, Serialize};

use crate::newtypes::{ChannelId, UserGroupId, UserId};

#[derive(Clone, Debug, Serialize)]
pub struct ListRequest {
    pub include_count: Option<bool>,
    pub include_disabled: Option<bool>,
    pub include_users: Option<bool>,
    pub team_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ListResponse {
    pub usergroups: Vec<UserGroup>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserGroup {
    pub id: UserGroupId,
    pub team_id: String,
    pub is_usergroup: bool,
    pub name: String,
    pub description: String,
    pub handle: String,
    pub is_external: bool,
    pub created_by: UserId,
    pub updated_by: UserId,
    pub prefs: UserGroupPrefs,
    pub user_count: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserGroupPrefs {
    pub channels: Vec<ChannelId>,
    // TODO: Find out what ID space this is.
    pub groups: Vec<String>,
}

#[derive(Serialize)]
pub struct ListUsersRequest {
    pub usergroup: UserGroupId,
}

#[derive(Debug, Deserialize)]
pub struct ListUsersResponse {
    pub users: Vec<UserId>,
}
