#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

use lazy_static::lazy_static;
pub mod auth;
pub mod game_profile;
pub mod resources;
pub mod error;

mod store;
mod schema;

embed_migrations!();

lazy_static! {
    static ref client: reqwest::Client = reqwest::Client::new();
    static ref config: Config = Config::new();
}

struct Config {
    launcher_path: String
}

impl Config {
    fn new() -> Self {
        let launcher_path = std::env::var("LAUNCHER_PATH");
        Self {
            launcher_path: launcher_path.unwrap_or_else(|_| Config::default().launcher_path )
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self { launcher_path:  format!("{}/liblauncher", dirs::data_dir().unwrap().to_str().unwrap()) }
    }
}

fn path_with_launcher(path: &str) -> String {
    format!("{}/{path}", config.launcher_path)
}

#[cfg(test)]
mod tests {
    use tracing::info;

    use crate::{game_profile::Profile, store};


    #[tokio::test]
    #[tracing_test::traced_test]
    async fn run() {
        /* 
        store::init_store().unwrap();
        info!("{}", crate::auth::microsoft::get_microsoft_auth_uri());
        let code = crate::auth::microsoft::await_token().await.unwrap();
        let account = crate::auth::new_microsoft_login(&code).await.unwrap();
        */
        
        info!("hola?");
        let versions = crate::resources::version::get_available_versions()
            .await
            .unwrap();
        let latest = versions.release().iter().find(|version| version.id == "1.18").unwrap().get_details().await.unwrap();
        info!("holados? {}", super::config.launcher_path);
        latest.download_client().await.unwrap();
        //latest.download_jdk().await.unwrap();
        latest.download_libraries(|_, _| {}).await.unwrap();
        latest.download_assets(|_, _| {}).await.unwrap();
        
        latest.extract_natives().unwrap();
        
        let profile = Profile::new(&latest, "1.19");
        profile.run(&store::get_accounts().unwrap()[0]).await.unwrap();

    }
}    
