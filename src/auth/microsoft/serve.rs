use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use warp::Filter;

pub async fn await_token() -> Option<String> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let filter = warp::any()
        .and(warp::query::query())
        .map(move |query_map: HashMap<String, String>| {
            return if let Some(token) = query_map.get("code") {
                tx.send(Some(token.clone())).unwrap();
                warp::reply::html("<html><head><title>Authentication successful</title></head><body><p>You can close this window.<p></body></html>")
            } else {
                tx.send(None).unwrap();
                warp::reply::html("<html><head><title>Authentication failed</title></head><body><p>Unknown error.</p></body></html>")
            }


        });

    let token = Arc::new(Mutex::new(None));
    let token_c = token.clone();
    let (_, server) = warp::serve(filter).bind_with_graceful_shutdown(([127, 0, 0, 1], 7575), async move {
        if let Some(t) = rx.recv().await {
            *token_c.lock().unwrap() = Some(t);
        }
    });

    server.await;

    let r = token.lock().unwrap().take().unwrap();
    r
}

#[cfg(test)]
mod tests {
    use tokio::test;
    use tracing_test::traced_test;
    use crate::auth::microsoft::serve::await_token;

    #[tokio::test]
    #[traced_test]
    async fn gets_code() {
        let token = await_token().await;
        assert_eq!(token, Some("nice".to_owned()))
    }
}