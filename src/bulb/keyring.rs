use std::process::Command;

use crate::bulb::model::is_token;
use crate::error::KeyringError;

#[derive(Clone, Debug)]
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new(service: String) -> Self {
        Self { service }
    }

    pub fn get(&self, account: &str) -> Result<Option<String>, KeyringError> {
        let entry = keyring::Entry::new(&self.service, account)
            .map_err(|source| KeyringError::Unavailable { source })?;

        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(self.legacy_lookup(account)),
            Err(source) => Err(KeyringError::Read { source }),
        }
    }

    pub fn set(&self, account: &str, secret: &str) -> Result<(), KeyringError> {
        let entry = keyring::Entry::new(&self.service, account)
            .map_err(|source| KeyringError::Unavailable { source })?;
        entry
            .set_password(secret)
            .map_err(|source| KeyringError::Write { source })
    }

    pub fn remove(&self, account: &str) -> Result<(), KeyringError> {
        let entry = keyring::Entry::new(&self.service, account)
            .map_err(|source| KeyringError::Unavailable { source })?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {
                self.legacy_clear(account);
                Ok(())
            }
            Err(source) => Err(KeyringError::Delete { source }),
        }
    }

    fn legacy_lookup(&self, account: &str) -> Option<String> {
        let output = Command::new("secret-tool")
            .args(["lookup", "service", &self.service, "account", account])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let secret = String::from_utf8(output.stdout).ok()?.trim().to_owned();
        if !is_token(&secret) {
            return None;
        }
        if self.set(account, &secret).is_ok() {
            self.legacy_clear(account);
        }
        Some(secret)
    }

    fn legacy_clear(&self, account: &str) {
        let _ = Command::new("secret-tool")
            .args(["clear", "service", &self.service, "account", account])
            .status();
    }
}
