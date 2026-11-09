use thiserror::Error;

use crate::bulb::keyring::KeyringStore;
use crate::bulb::model::{Bulb, BulbListIssue, validate_bulbs};
use crate::bulb::store::BulbStore;
use crate::error::{KeyringError, StoreError};

pub trait MetadataStore {
    fn load(&self) -> Result<Vec<Bulb>, StoreError>;
    fn save(&self, bulbs: &[Bulb]) -> Result<(), StoreError>;
}

pub trait TokenStore {
    fn get(&self, account: &str) -> Result<Option<String>, KeyringError>;
    fn set(&self, account: &str, secret: &str) -> Result<(), KeyringError>;
    fn remove(&self, account: &str) -> Result<(), KeyringError>;
}

impl MetadataStore for BulbStore {
    fn load(&self) -> Result<Vec<Bulb>, StoreError> {
        BulbStore::load(self)
    }

    fn save(&self, bulbs: &[Bulb]) -> Result<(), StoreError> {
        BulbStore::save(self, bulbs)
    }
}

impl TokenStore for KeyringStore {
    fn get(&self, account: &str) -> Result<Option<String>, KeyringError> {
        KeyringStore::get(self, account)
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), KeyringError> {
        KeyringStore::set(self, account, secret)
    }

    fn remove(&self, account: &str) -> Result<(), KeyringError> {
        KeyringStore::remove(self, account)
    }
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("The bulb list {0}")]
    Invalid(BulbListIssue),
    #[error("{0}")]
    Store(#[from] StoreError),
    #[error("{0}")]
    Token(#[from] KeyringError),
    #[error("That bulb is no longer in the list.")]
    Unknown,
    #[error("{primary} The token also could not be rolled back: {rollback}")]
    TokenRollback {
        #[source]
        primary: StoreError,
        rollback: KeyringError,
    },
    #[error("{primary} The bulb file also could not be rolled back: {rollback}")]
    RecordRollback {
        #[source]
        primary: KeyringError,
        rollback: StoreError,
    },
}

pub struct BulbRegistry<M, T> {
    metadata: M,
    tokens: T,
}

impl<M: MetadataStore, T: TokenStore> BulbRegistry<M, T> {
    pub fn new(metadata: M, tokens: T) -> Self {
        Self { metadata, tokens }
    }

    pub fn load(&self) -> Result<Vec<Bulb>, RegistryError> {
        Ok(self.metadata.load()?)
    }

    pub fn token_store(&self) -> &T {
        &self.tokens
    }

    pub fn add(
        &self,
        bulbs: &[Bulb],
        bulb: &Bulb,
        token: &str,
    ) -> Result<Vec<Bulb>, RegistryError> {
        let mut next = bulbs.to_vec();
        next.push(bulb.clone());
        validate_bulbs(&next).map_err(RegistryError::Invalid)?;

        // Write the token first so a crash can leave an orphan secret, never a
        // record without a token.
        self.tokens.set(&bulb.id, token)?;
        if let Err(primary) = self.metadata.save(&next) {
            return match self.tokens.remove(&bulb.id) {
                Ok(()) => Err(RegistryError::Store(primary)),
                Err(rollback) => Err(RegistryError::TokenRollback { primary, rollback }),
            };
        }

        Ok(next)
    }

    pub fn update(
        &self,
        bulbs: &[Bulb],
        bulb: &Bulb,
        token: Option<&str>,
    ) -> Result<Vec<Bulb>, RegistryError> {
        let index = bulbs
            .iter()
            .position(|existing| existing.id == bulb.id)
            .ok_or(RegistryError::Unknown)?;
        let mut next = bulbs.to_vec();
        match next.get_mut(index) {
            Some(slot) => *slot = bulb.clone(),
            None => return Err(RegistryError::Unknown),
        }
        validate_bulbs(&next).map_err(RegistryError::Invalid)?;

        let previous = match token {
            Some(_) => Some(self.tokens.get(&bulb.id)?),
            None => None,
        };
        if let Some(token) = token {
            self.tokens.set(&bulb.id, token)?;
        }

        if let Err(primary) = self.metadata.save(&next) {
            if let Some(previous) = previous {
                return match previous {
                    Some(previous) => match self.tokens.set(&bulb.id, &previous) {
                        Ok(()) => Err(RegistryError::Store(primary)),
                        Err(rollback) => Err(RegistryError::TokenRollback { primary, rollback }),
                    },
                    None => match self.tokens.remove(&bulb.id) {
                        Ok(()) => Err(RegistryError::Store(primary)),
                        Err(rollback) => Err(RegistryError::TokenRollback { primary, rollback }),
                    },
                };
            }
            return Err(RegistryError::Store(primary));
        }

        Ok(next)
    }

    pub fn remove(&self, bulbs: &[Bulb], id: &str) -> Result<Vec<Bulb>, RegistryError> {
        if !bulbs.iter().any(|bulb| bulb.id == id) {
            return Err(RegistryError::Unknown);
        }
        let next: Vec<Bulb> = bulbs.iter().filter(|bulb| bulb.id != id).cloned().collect();

        // Remove the record first so a crash can leave an orphan secret, never
        // a record without a token.
        if let Err(primary) = self.metadata.save(&next) {
            return Err(RegistryError::Store(primary));
        }
        if let Err(primary) = self.tokens.remove(id) {
            return match self.metadata.save(bulbs) {
                Ok(()) => Err(RegistryError::Token(primary)),
                Err(rollback) => Err(RegistryError::RecordRollback { primary, rollback }),
            };
        }

        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::rc::Rc;

    use super::{BulbRegistry, MetadataStore, RegistryError, TokenStore};
    use crate::bulb::model::{Bulb, BulbListIssue};
    use crate::error::{KeyringError, StoreError};

    const DESK_ID: &str = "00000000-0000-4000-8000-000000000001";
    const SHELF_ID: &str = "00000000-0000-4000-8000-000000000002";
    const TOKEN: &str = "0123456789abcdef0123456789abcdef";
    const OTHER_TOKEN: &str = "fedcba9876543210fedcba9876543210";

    #[derive(Default)]
    struct FakeState {
        bulbs: RefCell<Vec<Bulb>>,
        tokens: RefCell<HashMap<String, String>>,
        fail_save: Cell<bool>,
        fail_set: Cell<bool>,
        fail_remove: Cell<bool>,
        log: RefCell<Vec<&'static str>>,
    }

    struct FakeMetadata {
        state: Rc<FakeState>,
    }

    struct FakeTokens {
        state: Rc<FakeState>,
    }

    impl MetadataStore for FakeMetadata {
        fn load(&self) -> Result<Vec<Bulb>, StoreError> {
            Ok(self.state.bulbs.borrow().clone())
        }

        fn save(&self, bulbs: &[Bulb]) -> Result<(), StoreError> {
            self.state.log.borrow_mut().push("metadata.save");
            if self.state.fail_save.get() {
                return Err(write_error());
            }
            *self.state.bulbs.borrow_mut() = bulbs.to_vec();
            Ok(())
        }
    }

    impl TokenStore for FakeTokens {
        fn get(&self, account: &str) -> Result<Option<String>, KeyringError> {
            self.state.log.borrow_mut().push("token.get");
            Ok(self.state.tokens.borrow().get(account).cloned())
        }

        fn set(&self, account: &str, secret: &str) -> Result<(), KeyringError> {
            self.state.log.borrow_mut().push("token.set");
            if self.state.fail_set.get() {
                return Err(keyring_error());
            }
            self.state
                .tokens
                .borrow_mut()
                .insert(account.to_owned(), secret.to_owned());
            Ok(())
        }

        fn remove(&self, account: &str) -> Result<(), KeyringError> {
            self.state.log.borrow_mut().push("token.remove");
            if self.state.fail_remove.get() {
                return Err(keyring_error());
            }
            self.state.tokens.borrow_mut().remove(account);
            Ok(())
        }
    }

    fn write_error() -> StoreError {
        StoreError::Write {
            path: "bulbs.json".into(),
            source: std::io::Error::other("write failed"),
        }
    }

    fn keyring_error() -> KeyringError {
        KeyringError::Delete {
            source: keyring::Error::NoEntry,
        }
    }

    fn bulb(id: &str, name: &str, address: &str) -> Bulb {
        Bulb {
            id: id.to_owned(),
            name: name.to_owned(),
            address: address.to_owned(),
            model: None,
        }
    }

    fn desk() -> Bulb {
        bulb(DESK_ID, "Desk lamp", "192.168.1.42")
    }

    fn shelf() -> Bulb {
        bulb(SHELF_ID, "Shelf lamp", "192.168.1.43")
    }

    fn registry() -> (
        BulbRegistry<FakeMetadata, FakeTokens>,
        Rc<FakeState>,
        Vec<Bulb>,
    ) {
        let state = Rc::new(FakeState::default());
        *state.bulbs.borrow_mut() = vec![desk()];
        let registry = BulbRegistry::new(
            FakeMetadata {
                state: Rc::clone(&state),
            },
            FakeTokens {
                state: Rc::clone(&state),
            },
        );
        (registry, state, vec![desk()])
    }

    fn log(state: &FakeState) -> Vec<&'static str> {
        state.log.borrow().clone()
    }

    #[test]
    fn a_load_returns_the_saved_bulbs() -> Result<(), Box<dyn std::error::Error>> {
        let (registry, _state, bulbs) = registry();

        assert_eq!(registry.load()?, bulbs);
        Ok(())
    }

    #[test]
    fn an_add_writes_the_token_before_the_record() -> Result<(), Box<dyn std::error::Error>> {
        let (registry, state, bulbs) = registry();
        let next = registry.add(&bulbs, &shelf(), TOKEN)?;

        assert_eq!(next, vec![desk(), shelf()]);
        assert_eq!(log(&state), ["token.set", "metadata.save"]);
        assert_eq!(
            state.tokens.borrow().get(SHELF_ID).map(String::as_str),
            Some(TOKEN)
        );
        assert_eq!(state.bulbs.borrow().as_slice(), next.as_slice());
        Ok(())
    }

    #[test]
    fn an_add_rolls_the_token_back_when_the_record_fails() -> Result<(), Box<dyn std::error::Error>>
    {
        let (registry, state, bulbs) = registry();
        state.fail_save.set(true);

        assert!(matches!(
            registry.add(&bulbs, &shelf(), TOKEN),
            Err(RegistryError::Store(_))
        ));
        assert_eq!(log(&state), ["token.set", "metadata.save", "token.remove"]);
        assert!(state.tokens.borrow().get(SHELF_ID).is_none());
        Ok(())
    }

    #[test]
    fn an_add_leaves_the_record_when_the_token_write_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let (registry, state, bulbs) = registry();
        state.fail_set.set(true);

        assert!(matches!(
            registry.add(&bulbs, &shelf(), TOKEN),
            Err(RegistryError::Token(_))
        ));
        assert_eq!(log(&state), ["token.set"]);
        assert_eq!(state.bulbs.borrow().as_slice(), bulbs.as_slice());
        assert!(state.tokens.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn an_update_leaves_the_pair_when_the_token_write_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let (registry, state, bulbs) = registry();
        state
            .tokens
            .borrow_mut()
            .insert(DESK_ID.to_owned(), TOKEN.to_owned());
        state.fail_set.set(true);

        assert!(matches!(
            registry.update(&bulbs, &desk(), Some(OTHER_TOKEN)),
            Err(RegistryError::Token(_))
        ));
        assert_eq!(log(&state), ["token.get", "token.set"]);
        assert_eq!(
            state.tokens.borrow().get(DESK_ID).map(String::as_str),
            Some(TOKEN)
        );
        assert_eq!(state.bulbs.borrow().as_slice(), bulbs.as_slice());
        Ok(())
    }

    #[test]
    fn an_update_restores_the_previous_token_when_the_record_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let (registry, state, bulbs) = registry();
        state
            .tokens
            .borrow_mut()
            .insert(DESK_ID.to_owned(), TOKEN.to_owned());
        state.fail_save.set(true);

        assert!(matches!(
            registry.update(&bulbs, &desk(), Some(OTHER_TOKEN)),
            Err(RegistryError::Store(_))
        ));
        assert_eq!(
            log(&state),
            ["token.get", "token.set", "metadata.save", "token.set"]
        );
        assert_eq!(
            state.tokens.borrow().get(DESK_ID).map(String::as_str),
            Some(TOKEN)
        );
        Ok(())
    }

    #[test]
    fn an_update_removes_a_new_token_when_the_record_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let (registry, state, bulbs) = registry();
        state.fail_save.set(true);

        assert!(matches!(
            registry.update(&bulbs, &desk(), Some(OTHER_TOKEN)),
            Err(RegistryError::Store(_))
        ));
        assert_eq!(
            log(&state),
            ["token.get", "token.set", "metadata.save", "token.remove"]
        );
        assert!(state.tokens.borrow().get(DESK_ID).is_none());
        Ok(())
    }

    #[test]
    fn an_update_without_a_token_leaves_the_token_alone() -> Result<(), Box<dyn std::error::Error>>
    {
        let (registry, state, bulbs) = registry();
        state
            .tokens
            .borrow_mut()
            .insert(DESK_ID.to_owned(), TOKEN.to_owned());
        let mut renamed = desk();
        renamed.name = "Desk".to_owned();

        let next = registry.update(&bulbs, &renamed, None)?;

        assert_eq!(next, vec![renamed]);
        assert_eq!(log(&state), ["metadata.save"]);
        assert_eq!(
            state.tokens.borrow().get(DESK_ID).map(String::as_str),
            Some(TOKEN)
        );
        Ok(())
    }

    #[test]
    fn a_remove_writes_the_record_before_the_token() -> Result<(), Box<dyn std::error::Error>> {
        let (registry, state, bulbs) = registry();
        state
            .tokens
            .borrow_mut()
            .insert(DESK_ID.to_owned(), TOKEN.to_owned());

        let next = registry.remove(&bulbs, DESK_ID)?;

        assert!(next.is_empty());
        assert_eq!(log(&state), ["metadata.save", "token.remove"]);
        assert!(state.tokens.borrow().get(DESK_ID).is_none());
        assert!(state.bulbs.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn a_remove_restores_the_record_when_the_token_delete_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let (registry, state, bulbs) = registry();
        state
            .tokens
            .borrow_mut()
            .insert(DESK_ID.to_owned(), TOKEN.to_owned());
        state.fail_remove.set(true);

        assert!(matches!(
            registry.remove(&bulbs, DESK_ID),
            Err(RegistryError::Token(_))
        ));
        assert_eq!(
            log(&state),
            ["metadata.save", "token.remove", "metadata.save"]
        );
        assert_eq!(state.bulbs.borrow().as_slice(), bulbs.as_slice());
        assert_eq!(
            state.tokens.borrow().get(DESK_ID).map(String::as_str),
            Some(TOKEN)
        );
        Ok(())
    }

    #[test]
    fn a_remove_leaves_the_pair_when_the_record_fails() -> Result<(), Box<dyn std::error::Error>> {
        let (registry, state, bulbs) = registry();
        state
            .tokens
            .borrow_mut()
            .insert(DESK_ID.to_owned(), TOKEN.to_owned());
        state.fail_save.set(true);

        assert!(matches!(
            registry.remove(&bulbs, DESK_ID),
            Err(RegistryError::Store(_))
        ));
        assert_eq!(log(&state), ["metadata.save"]);
        assert_eq!(state.bulbs.borrow().as_slice(), bulbs.as_slice());
        assert_eq!(
            state.tokens.borrow().get(DESK_ID).map(String::as_str),
            Some(TOKEN)
        );
        Ok(())
    }

    #[test]
    fn duplicate_names_and_addresses_are_rejected_before_writing() {
        let (registry, state, bulbs) = registry();
        let mut duplicate_name = shelf();
        duplicate_name.name = "desk LAMP".to_owned();
        assert!(matches!(
            registry.add(&bulbs, &duplicate_name, TOKEN),
            Err(RegistryError::Invalid(BulbListIssue::DuplicateName { .. }))
        ));

        let mut duplicate_address = shelf();
        duplicate_address.address = "192.168.1.42".to_owned();
        assert!(matches!(
            registry.add(&bulbs, &duplicate_address, TOKEN),
            Err(RegistryError::Invalid(
                BulbListIssue::DuplicateAddress { .. }
            ))
        ));

        assert!(log(&state).is_empty());
        assert!(state.tokens.borrow().is_empty());
        assert_eq!(state.bulbs.borrow().as_slice(), bulbs.as_slice());
    }

    #[test]
    fn an_unknown_bulb_cannot_be_updated_or_removed() {
        let (registry, state, bulbs) = registry();
        let missing = bulb(
            "00000000-0000-4000-8000-000000000099",
            "Missing",
            "192.168.1.99",
        );

        assert!(matches!(
            registry.update(&bulbs, &missing, Some(TOKEN)),
            Err(RegistryError::Unknown)
        ));
        assert!(matches!(
            registry.remove(&bulbs, missing.id.as_str()),
            Err(RegistryError::Unknown)
        ));
        assert!(log(&state).is_empty());
    }
}
