use movo_core::client::models::UserProfile;
use movo_core::client::RezkaClient;
use movo_core::storage::settings::AppSettings;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Identity of the signed-in account at a point in time.
///
/// Every request records the account it started under and is compared against
/// the current one before its result is applied, so a sign-in, sign-out or
/// expired session cannot leave another account's data on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub generation: u64,
    pub user_id: String,
}

/// Application state shared by every view on the main thread.
pub struct AppState {
    /// Cloned into background tasks; the client itself is internally shared.
    pub client: Arc<RezkaClient>,
    settings: RefCell<AppSettings>,
}

impl AppState {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            client: Arc::new(RezkaClient::new()),
            settings: RefCell::new(AppSettings::load()),
        })
    }

    pub fn settings(&self) -> AppSettings {
        self.settings.borrow().clone()
    }

    pub fn set_settings(&self, settings: AppSettings) {
        *self.settings.borrow_mut() = settings;
    }

    pub fn user(&self) -> Option<UserProfile> {
        self.client.user()
    }

    pub fn account(&self) -> Option<Account> {
        account_of(&self.client)
    }

    /// Whether a result produced under `account` still belongs on screen.
    ///
    /// Signed-out browsing is legitimate, so `None` before and after is
    /// accepted; any change of account or generation is not.
    pub fn accepts(&self, account: &Option<Account>) -> bool {
        &self.account() == account
    }
}

pub fn account_of(client: &RezkaClient) -> Option<Account> {
    client.user().map(|user| Account {
        generation: client.account_generation(),
        user_id: user.user_id,
    })
}
