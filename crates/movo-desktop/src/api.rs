use crate::i18n::tr;
use crate::state::{account_of, Account};
use movo_core::client::RezkaClient;
use movo_core::error::{ClientError, ErrorKind};
use std::future::Future;
use std::sync::Arc;

/// What to tell the user about a failed request. A provider's own refusal, such
/// as a wrong password, is passed on; the transport details behind the others
/// mean nothing on screen and go to the log.
pub fn describe(error: &ClientError) -> String {
    let explained = match error.kind {
        ErrorKind::Timeout => tr("The provider took too long to answer. Try again in a moment."),
        ErrorKind::Network => tr("Could not reach the provider. Check your connection."),
        ErrorKind::Provider => tr("The provider is not answering properly. Try again later."),
        ErrorKind::Session => tr("Your session has ended. Sign in again to continue."),
        ErrorKind::Malformed => tr("The provider sent a page this version of Movo cannot read."),
        ErrorKind::Other => return error.message.clone(),
    };
    log::warn!("request failed: {}", error.message);
    explained.to_string()
}

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

#[cfg(test)]
mod tests {
    use super::describe;
    use movo_core::error::{ClientError, ErrorKind};

    #[test]
    fn transport_details_stay_off_the_screen() {
        let error = ClientError::new(
            ErrorKind::Provider,
            "GET failed: HTTP 503; content-type text/html; empty body (0 bytes)",
        );
        assert!(!describe(&error).contains("503"));
    }

    #[test]
    fn a_refusal_in_the_providers_words_is_passed_on() {
        let error = ClientError::new(ErrorKind::Other, "Неверный логин или пароль");
        assert_eq!(describe(&error), "Неверный логин или пароль");
    }
}
