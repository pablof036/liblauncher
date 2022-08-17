mod mojang;
mod profile;
pub mod microsoft;

pub use crate::{error::{Error, Result}, store::{models::Account, store_account}};

use self::{microsoft::microsoft_login, profile::get_profile, mojang::mojang_login};

pub async fn new_mojang_login(username: &str, password: &str) -> Result<Account> {
    let access_token = mojang_login(username, password).await?;
    store_new_account(&access_token).await
}

pub async fn new_microsoft_login(code: &str) -> Result<Account> {
    let access_token = microsoft_login(code).await?;
    store_new_account(&access_token).await
}

pub async fn new_offline_login(username: &str) -> Result<Account> {
    let account = Account {
        username: username.to_owned(),
        ..Default::default()
    };
    store_account(&account)?;
    Ok(account)
}

async fn store_new_account(access_token: &str) -> Result<Account> {
    let profile = get_profile(&access_token).await?;
    let account = Account {
        id: None,
        access_token: access_token.to_string(),
        account_uuid: profile.id,
        username: profile.name,
        client_id: String::from("liblauncher"), 
    };
    store_account(&account)?;
    Ok(account)
}

pub async fn get_accounts() -> Result<Vec<Account>> {
    crate::store::get_accounts()
}


#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    use super::microsoft::await_token;
    use super::new_microsoft_login;

    #[tokio::test]
    #[traced_test]
    async fn successful_login() {
        let code = await_token().await.unwrap();
        new_microsoft_login(&code).await.unwrap();
    }
}

