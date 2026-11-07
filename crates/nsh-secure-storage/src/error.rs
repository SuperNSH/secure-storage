//! The errors a store reports. They carry a stable code and the key's name, never a value.

use std::fmt;

/// What went wrong, as a stable code; see [`SecureStorageError::code`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// The store is locked or not readable right now; retry after unlock.
    Locked,
    /// The platform cannot do what was asked here (no store, or an option not built yet).
    NotAvailable,
    /// The platform's key for this data was permanently invalidated.
    KeyInvalidated,
    /// Stored data cannot be decoded, or failed its integrity check.
    Corrupt,
    /// The user dismissed an authentication prompt.
    AuthCancelled,
    /// Authentication was attempted and failed.
    AuthFailed,
    /// The app is not allowed to use the store group it asked for.
    MissingEntitlement,
    /// The key is empty or contains a NUL character.
    InvalidKey,
    /// The key was invalidated, the store was reset, and the retry failed too.
    ResetFailed,
    /// The platform store failed; `status` holds its own code where it has one.
    Backend,
}

/// A failed store operation.
///
/// `Display` and `Debug` name the code, the key and the platform status. Neither holds a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecureStorageError {
    kind: ErrorKind,
    key: Option<String>,
    status: Option<i32>,
}

impl SecureStorageError {
    /// An error of `kind`, without a key or a platform status.
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            key: None,
            status: None,
        }
    }

    /// The same error, naming the key it concerns.
    #[must_use]
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// The same error, carrying the platform's own status code.
    #[must_use]
    pub fn with_status(mut self, status: i32) -> Self {
        self.status = Some(status);
        self
    }

    /// What went wrong.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The key the error concerns, when there is one.
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// The platform's status code, when the platform gave one.
    pub fn status(&self) -> Option<i32> {
        self.status
    }

    /// The stable code, such as `ERR_SECURE_STORAGE_LOCKED`. Apps match on this.
    pub fn code(&self) -> &'static str {
        match self.kind {
            ErrorKind::Locked => "ERR_SECURE_STORAGE_LOCKED",
            ErrorKind::NotAvailable => "ERR_SECURE_STORAGE_NOT_AVAILABLE",
            ErrorKind::KeyInvalidated => "ERR_SECURE_STORAGE_KEY_INVALIDATED",
            ErrorKind::Corrupt => "ERR_SECURE_STORAGE_CORRUPT",
            ErrorKind::AuthCancelled => "ERR_SECURE_STORAGE_AUTH_CANCELLED",
            ErrorKind::AuthFailed => "ERR_SECURE_STORAGE_AUTH_FAILED",
            ErrorKind::MissingEntitlement => "ERR_SECURE_STORAGE_MISSING_ENTITLEMENT",
            ErrorKind::InvalidKey => "ERR_SECURE_STORAGE_INVALID_KEY",
            ErrorKind::ResetFailed => "ERR_SECURE_STORAGE_RESET_FAILED",
            ErrorKind::Backend => "ERR_SECURE_STORAGE_BACKEND",
        }
    }

    /// What to do about it, in one sentence. Never includes a value.
    pub fn message(&self) -> &'static str {
        match self.kind {
            ErrorKind::Locked => "The store is locked. Unlock the device and try again.",
            ErrorKind::NotAvailable => {
                "This platform cannot do this here. Check the options and the platform's support."
            }
            ErrorKind::KeyInvalidated => {
                "The key for this data was invalidated, so the data cannot be read. Delete the key and write it again."
            }
            ErrorKind::Corrupt => "The stored data is damaged and cannot be decoded.",
            ErrorKind::AuthCancelled => "The user cancelled the authentication prompt.",
            ErrorKind::AuthFailed => "Authentication failed.",
            ErrorKind::MissingEntitlement => {
                "The app is not signed with the keychain access group it asks for. Add that group to the app's keychain-access-groups entitlement and sign the app with it."
            }
            ErrorKind::InvalidKey => "The key must not be empty or contain a NUL character.",
            ErrorKind::ResetFailed => {
                "The key was invalidated and the store was reset, but the data still cannot be used."
            }
            ErrorKind::Backend => "The platform's secret store failed.",
        }
    }
}

impl fmt::Display for SecureStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())?;
        if let Some(key) = &self.key {
            write!(f, " (key {key:?})")?;
        }
        if let Some(status) = self.status {
            write!(f, " (status {status})")?;
        }
        Ok(())
    }
}

impl std::error::Error for SecureStorageError {}
