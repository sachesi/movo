use crate::state::{account_of, Account};
use movo_core::client::RezkaClient;
use std::future::Future;
use std::sync::Arc;

/// A request result together with the account it was produced under.
#[derive(Debug)]
pub struct Guarded<T> {
    pub account: Option<Account>,
    pub result: Result<T, String>,
}

/// Run `request` on the shared runtime, recording the account it ran under.
///
/// A panic inside movo-core is reported as a failed request instead of
/// unwinding into the GTK main loop.
pub async fn guarded<T, F, Fut>(client: Arc<RezkaClient>, request: F) -> Guarded<T>
where
    F: FnOnce(Arc<RezkaClient>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, String>> + Send,
    T: Send + 'static,
{
    let account = account_of(&client);
    let result = relm4::spawn(async move { request(client).await }).await;
    Guarded {
        account,
        result: result.unwrap_or_else(|_| Err("The request stopped unexpectedly".to_string())),
    }
}
