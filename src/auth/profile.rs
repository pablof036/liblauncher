use reqwest::StatusCode;
use serde::{Serialize, Deserialize};
use crate::error::{Error, ProfileError};
use crate::client;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
}

pub async fn get_profile(access_token: &str) -> Result<Profile, Error> {
    let response = client.get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(access_token)
        .send()
        .await?;

    return if response.status() == StatusCode::NOT_FOUND {
        Err(Error::ProfileError(ProfileError::ProfileNotFound))
    } else if response.status() == StatusCode::FORBIDDEN {
        Err(Error::ProfileError(ProfileError::BadAccessToken))
    } else {
        Ok(response.json().await?)
    }
}
