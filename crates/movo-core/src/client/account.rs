use super::models::UserProfile;
use super::{auth, blocking, RestoreError, RezkaClient};
use crate::error::ClientError;
use std::sync::PoisonError;

impl RezkaClient {
    pub fn export_session(&self) -> Result<String, ClientError> {
        self.ensure_signed_in()?;
        self.session.export_session()
    }

    pub async fn import_session(&self, secret: &str) -> Result<UserProfile, RestoreError> {
        self.session
            .import_session(secret)
            .map_err(RestoreError::rejected)?;
        let profile = auth::check_profile(&self.session)
            .await
            .map_err(RestoreError::retryable)?
            .ok_or_else(|| RestoreError::rejected("Stored session has expired".to_string()))?;
        self.set_account(Some(profile.clone()));
        Ok(profile)
    }

    pub async fn login(
        &self,
        email_or_login: &str,
        password: &str,
    ) -> Result<UserProfile, ClientError> {
        let mut profile = auth::login(&self.session, email_or_login, password).await?;
        let mut settings = blocking(
            "Loading settings",
            crate::storage::settings::AppSettings::load,
        )
        .await?;
        settings.user_id = Some(profile.user_id.clone());
        let saved = settings.clone();
        let save_ok = blocking("Saving settings", move || saved.save())
            .await
            .is_ok_and(|result| result.is_ok());
        if !save_ok {
            profile.is_session_persistent = false;
        }
        self.set_account(Some(profile.clone()));
        Ok(profile)
    }

    pub async fn restore_session(&self) -> Result<Option<UserProfile>, ClientError> {
        let mut settings = blocking(
            "Loading settings",
            crate::storage::settings::AppSettings::load,
        )
        .await?;
        let Some(user_id) = settings.user_id.clone() else {
            return Ok(None);
        };
        let session = self.session.clone();
        let restored = blocking("Restoring the session", move || {
            session.restore_session(&user_id)
        })
        .await??;
        if restored {
            return self.verify_restored_session(&mut settings).await;
        }
        settings.user_id = None;
        let saved = settings.clone();
        blocking("Saving settings", move || saved.save()).await??;
        self.set_account(None);
        Ok(None)
    }

    async fn verify_restored_session(
        &self,
        settings: &mut crate::storage::settings::AppSettings,
    ) -> Result<Option<UserProfile>, ClientError> {
        match auth::check_profile(&self.session).await {
            Ok(Some(profile)) => {
                self.set_account(Some(profile.clone()));
                Ok(Some(profile))
            }
            Ok(None) => {
                self.set_account(None);
                let user_id = settings.user_id.take();
                let session = self.session.clone();
                let _ = blocking("Clearing the session", move || {
                    session.clear_session(user_id.as_deref())
                })
                .await;
                let saved = settings.clone();
                blocking("Saving settings", move || saved.save()).await??;
                Ok(None)
            }
            Err(error) => {
                self.set_account(None);
                let user_id = settings.user_id.take();
                let session = self.session.clone();
                let _ = blocking("Clearing the session", move || {
                    session.clear_session(user_id.as_deref())
                })
                .await;
                let saved = settings.clone();
                blocking("Saving settings", move || saved.save()).await??;
                Err(ClientError::from(error))
            }
        }
    }

    pub async fn logout(&self) -> Result<(), ClientError> {
        let user_id = self
            .account
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .user
            .as_ref()
            .map(|user| user.user_id.clone());
        let session = self.session.clone();
        let logout_id = user_id.clone();
        let result = blocking("Signing out", move || {
            auth::logout(&session, logout_id.as_deref())
        })
        .await
        .and_then(|inner| inner.map_err(ClientError::from));
        self.set_account(None);
        let mut settings = blocking(
            "Loading settings",
            crate::storage::settings::AppSettings::load,
        )
        .await?;
        settings.user_id = None;
        let saved = settings.clone();
        let settings_cleared = blocking("Saving settings", move || saved.save())
            .await
            .is_ok_and(|inner| inner.is_ok());
        match (result, settings_cleared) {
            (Ok(()), true) => Ok(()),
            (Err(error), true) => Err(error),
            (Ok(()), false) => Err(ClientError::from(
                "Account selection could not be cleared".to_string(),
            )),
            (Err(_), false) => Err(ClientError::from(
                "Stored session and account selection could not be fully cleared".to_string(),
            )),
        }
    }
}
