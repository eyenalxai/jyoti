use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Could not read {path}. Check its permissions and try again.")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} does not match the bulb file format. Fix or remove it, then refresh.")]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("{path} has two bulbs with the same ID. Remove one entry and add it again.")]
    DuplicateId { path: PathBuf },
    #[error("{path} has two bulbs named {name}. Rename or remove one entry, then refresh.")]
    DuplicateName { path: PathBuf, name: String },
    #[error("{path} has two bulbs at {address}. Remove one entry, then refresh.")]
    DuplicateAddress { path: PathBuf, address: String },
    #[error(
        "{path} contains a token. Tokens live in the OS keyring; remove the token field, then refresh."
    )]
    TokenKey { path: PathBuf },
    #[error("{message}")]
    Invalid { path: PathBuf, message: String },
    #[error("Could not write {path}. Check its permissions.")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum KeyringError {
    #[error("Could not reach the OS keyring. Unlock it and try again.")]
    Unavailable {
        #[source]
        source: keyring::Error,
    },
    #[error("The OS keyring refused to read this bulb's token. Unlock it and try again.")]
    Read {
        #[source]
        source: keyring::Error,
    },
    #[error("The OS keyring refused to save this bulb's token. Unlock it and try again.")]
    Write {
        #[source]
        source: keyring::Error,
    },
    #[error("The OS keyring refused to delete this bulb's token.")]
    Delete {
        #[source]
        source: keyring::Error,
    },
}

#[derive(Debug, Error, Clone)]
pub enum MiioError {
    #[error("The bulb at {address} did not answer. Check its power and network.")]
    Timeout { address: String },
    #[error("Could not talk to the bulb at {address}. Check it is powered and on this network.")]
    Network { address: String, message: String },
    #[error("The bulb at {address} sent a reply jyoti could not read.")]
    Protocol { address: String, message: String },
    #[error("The bulb at {address} refused the command: {message}")]
    Device { address: String, message: String },
    #[error(
        "The token stored for the bulb at {address} is not valid. Edit the bulb and enter its token again."
    )]
    Token { address: String },
    #[error("{message}")]
    Value { address: String, message: String },
    #[error("{message}")]
    Address { address: String, message: String },
}

impl MiioError {
    pub fn is_device(&self) -> bool {
        matches!(self, MiioError::Device { .. })
    }
}
