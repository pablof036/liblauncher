mod serve;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
pub use serve::await_token;

use crate::{client, error::Result};

#[derive(Debug, Deserialize)]
struct MicrosoftResponse {
    access_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct XboxLivePayload<'a> {
    properties: XboxLiveProperties<'a>,
    relying_party: &'a str,
    token_type: &'a str,
}

impl<'a> Default for XboxLivePayload<'a> {
    fn default() -> Self {
        Self {
            properties: Default::default(),
            relying_party: "http://auth.xboxlive.com",
            token_type: "JWT",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct XboxLiveProperties<'a> {
    auth_method: &'a str,
    site_name: &'a str,
    rps_ticket: &'a str,
}

impl<'a> Default for XboxLiveProperties<'a> {
    fn default() -> Self {
        Self {
            auth_method: "RPS",
            site_name: "user.auth.xboxlive.com",
            rps_ticket: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxResponse {
    token: String,
    display_claims: HashMap<String, Vec<Xui>>,
}

#[derive(Debug, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct XstsPayload<'a> {
    properties: XstsProperties<'a>,
    relying_party: &'a str,
    token_type: &'a str,
}

impl<'a> Default for XstsPayload<'a> {
    fn default() -> Self {
        Self {
            properties: Default::default(),
            relying_party: "rp://api.minecraftservices.com/",
            token_type: "JWT",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct XstsProperties<'a> {
    sandbox_id: &'a str,
    user_tokens: Vec<&'a str>,
}

impl<'a> Default for XstsProperties<'a> {
    fn default() -> Self {
        Self {
            sandbox_id: "RETAIL",
            user_tokens: Default::default(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct MojangPayload<'a> {
    identity_token: &'a str,
    ensure_legacy_enabled: bool,
}

impl<'a> Default for MojangPayload<'a> {
    fn default() -> Self {
        Self {
            identity_token: Default::default(),
            ensure_legacy_enabled: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct MojangResponse {
    access_token: String,
}

pub async fn get_microsoft_auth_uri() -> String {
    format!(
        "https://login.live.com/oauth20_authorize.srf?client_id={}&response_type=code&redirect_uri=http://localhost:7575&scope=XboxLive.signin%20offline_access",
        include_str!("../../../secrets/client_id")
    )
}

pub async fn microsoft_login(code: &str) -> Result<String> {
    let access_token = client
        .post("https://login.live.com/oauth20_token.srf")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "client_id={}&code={}&grant_type=authorization_code&&redirect_uri={}",
            include_str!("../../../secrets/client_id"),
            code,
            "http://localhost:7575"
        ))
        .send()
        .await?
        .json::<MicrosoftResponse>()
        .await?
        .access_token;

    let xbox_payload = XboxLivePayload {
        properties: XboxLiveProperties {
            rps_ticket: &format!("d={}", &access_token),
            ..Default::default()
        },
        ..Default::default()
    };

    let XboxResponse {
        token,
        display_claims,
    } = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Accept", "application/json")
        .json(&xbox_payload)
        .send()
        .await?
        .json::<XboxResponse>()
        .await?;

    let uhs = display_claims.get("xui").unwrap().first().unwrap().uhs.clone();

    let xsts_payload = XstsPayload {
        properties: XstsProperties {
            user_tokens: vec![&token],
            ..Default::default()
        },
        ..Default::default()
    };

    let XboxResponse { token, .. } = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&xsts_payload)
        .send()
        .await?
        .json::<XboxResponse>()
        .await?;

    let mojang_payload = MojangPayload {
        identity_token: &format!("XBL3.0 x={};{}", uhs, token),
        ..Default::default()
    };

    let access_token = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&mojang_payload)
        .send()
        .await?
        .json::<MojangResponse>()
        .await?
        .access_token;

    Ok(access_token)
}
