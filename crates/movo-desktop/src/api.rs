use crate::i18n::tr;
use crate::state::{account_of, Account};
use movo_core::client::RezkaClient;
use movo_core::error::ClientError;
use std::future::Future;
use std::sync::Arc;

/// A request result together with the account it was produced under.
#[derive(Debug)]
pub struct Guarded<T> {
    pub account: Option<Account>,
    pub result: Result<T, ClientError>,
}

/// Run `request` on the shared runtime, recording the account it ran under.
///
/// A panic inside movo-core is reported as a failed request instead of
/// unwinding into the GTK main loop.
pub async fn guarded<T, F, Fut>(client: Arc<RezkaClient>, request: F) -> Guarded<T>
where
    F: FnOnce(Arc<RezkaClient>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, ClientError>> + Send,
    T: Send + 'static,
{
    let account = account_of(&client);
    guarded_as(client, account, request).await
}

/// Run a request only if the account captured by the caller is still current.
/// This closes the gap between a widget scheduling work and the worker starting
/// after a sign-in, sign-out, or account switch.
pub async fn guarded_as<T, F, Fut>(
    client: Arc<RezkaClient>,
    account: Option<Account>,
    request: F,
) -> Guarded<T>
where
    F: FnOnce(Arc<RezkaClient>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, ClientError>> + Send,
    T: Send + 'static,
{
    let expected = account.clone();
    let result = relm4::spawn(async move {
        if expected.is_some() && account_of(&client) != expected {
            return Err(ClientError::from(
                tr("The account changed before the request started").to_string(),
            ));
        }
        request(client).await
    })
    .await;
    Guarded {
        account,
        result: result.unwrap_or_else(|_| {
            Err(ClientError::from(
                tr("The request stopped unexpectedly").to_string(),
            ))
        }),
    }
}
