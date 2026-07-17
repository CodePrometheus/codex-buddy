use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by codex-buddy-core.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Json(serde_json::Error),
    /// auth.json is missing required fields or is malformed.
    InvalidAuth(String),
    /// The requested account alias does not exist.
    AccountNotFound(String),
    /// The account alias (or its account key) already exists.
    AccountExists(String),
    /// `cli_auth_credentials_store` is not `file` (keyring / auto / ephemeral).
    UnsupportedCredentialStore(String),
    /// The registry schema version is newer than this tool supports.
    RegistrySchemaTooNew {
        found: u32,
        supported: u32,
    },
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::Json(e) => write!(f, "json error: {e}"),
            Error::InvalidAuth(m) => write!(f, "invalid auth.json: {m}"),
            Error::AccountNotFound(a) => write!(f, "account not found: {a}"),
            Error::AccountExists(a) => write!(f, "account already exists: {a}"),
            Error::UnsupportedCredentialStore(m) => {
                write!(f, "unsupported credential store (must be `file`): {m}")
            }
            Error::RegistrySchemaTooNew { found, supported } => write!(
                f,
                "registry schema too new (file is {found}, this tool supports {supported}); upgrade codex-buddy"
            ),
            Error::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}
