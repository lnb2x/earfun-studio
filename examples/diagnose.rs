//! Read-only connection diagnostic. Never sends setting changes or disconnects audio.
use earfun_studio::backend::BackendCore;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let core = BackendCore::new(Arc::new(|event| println!("{event:?}")));
    let address = std::env::args().nth(1).unwrap_or_default();
    let result = tokio::time::timeout(Duration::from_secs(100), async {
        core.auto_connect(&address).await
    })
    .await;
    println!("RESULT: {result:?}");
    core.disconnect().await;
    if !matches!(result, Ok(Ok(()))) {
        std::process::exit(1);
    }
}
