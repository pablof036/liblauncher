use liblauncher::{
    auth::{
        microsoft::{await_token, get_microsoft_auth_uri},
        new_microsoft_login,
    },
    game_profile::Profile,
    resources::version::get_available_versions,
};

use tracing::info;

#[tokio::test]
#[tracing_test::traced_test]
async fn run_game() {
    info!("{}", get_microsoft_auth_uri());
    let code = await_token().await.unwrap();
    let account = new_microsoft_login(&code).await.unwrap();

    let versions = get_available_versions().await.unwrap();

    let latest = versions.latest_release().get_details().await.unwrap();
    latest.download_client().await.unwrap();
    latest.download_jdk().await.unwrap();
    latest.download_libraries(|_, _| {}).await.unwrap();
    latest.download_assets(|_, _| {}).await.unwrap();

    latest.extract_natives().unwrap();

    let profile = Profile::new(&latest, "1.19");
    profile
        .run(&account)
        .await
        .unwrap();
}
