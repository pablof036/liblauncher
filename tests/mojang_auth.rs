use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn mojang_auth() {
    liblauncher::auth::new_mojang_login(include_str!("../secrets/test_user"), include_str!("../secrets/test_password"))
        .await.unwrap();    
}