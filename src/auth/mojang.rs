use reqwest::{Response, StatusCode};

use serde::{Serialize, Deserialize};
use tracing::{error, info};
use crate::error::{Error, Result, MojangAuthError};
use crate::client;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticatePayload<'a> {
    username: &'a str,
    password: &'a str,
    client_token: Option<String>,
    request_user: bool,
}

impl<'a> Default for AuthenticatePayload<'a> {
    fn default() -> Self {
        Self {
            username: "",
            password: "",
            client_token: None,
            request_user: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticateResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    error: String,
    error_message: String,
    cause: Option<String>
}



pub async fn mojang_login(username: &str, password: &str) -> Result<String> {
    let payload = AuthenticatePayload {
        username,
        password,
        ..Default::default()
    };

    info!("trying Mojang authentication with username {}", payload.username);

    let response = client
        .post("https://authserver.mojang.com/authenticate")
        .json(&payload)
        .send()
        .await?;

    return if response.status() == StatusCode::OK {
        info!("authentication for username {} successful", payload.username);

        let access_token = response.json::<AuthenticateResponse>().await?.access_token;

        Ok(access_token)
    } else {
        let error: ErrorResponse = response.json().await?;
        Err(Error::MojangAuthError(get_error(error)))
    }
}

fn get_error(error: ErrorResponse) -> MojangAuthError {
    let error = match error.error.as_str() {
        "ForbiddenOperationException" => {
            match error.cause {
                Some(str) => match str.as_str() {
                    "UserMigratedException" => MojangAuthError::UsernameMigrated,
                    "InvalidCredentials" => MojangAuthError::RateLimited,
                    "Forbidden" => MojangAuthError::BadPassword,
                    _ => MojangAuthError::Unknown
                }
                None => MojangAuthError::InvalidCredentials,
            }
        },
        "ResourceException" | "GoneException" => MojangAuthError::AccountMigrated,
        _ => MojangAuthError::Unknown
    };

    error!("Mojang authentication error: {:?}", error);

    error
}

#[cfg(test)]
mod tests {
    use crate::auth::mojang::{mojang_login, MojangAuthError};
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn successful_auth() {
        let result = mojang_login(include_str!("../../secrets/test_user"), include_str!("../../secrets/test_password"))
            .await;

        result.unwrap();
    }
}