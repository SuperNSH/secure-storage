//! Glue between `keyring-core` and this crate, shared by the Windows and Linux backends.

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use keyring_core::Error as KeyringError;

use crate::error::{ErrorKind, SecureStorageError};

/// The error for a failed `keyring-core` call. The platform's text is dropped, so no value,
/// key or path reaches a log.
pub(crate) fn map_error(error: KeyringError) -> SecureStorageError {
    let kind = match &error {
        KeyringError::NoStorageAccess(inner) => no_storage_access(inner.as_ref()),
        KeyringError::PlatformFailure(inner) => platform_failure(inner.as_ref()),
        KeyringError::BadEncoding(_)
        | KeyringError::BadDataFormat(..)
        | KeyringError::BadStoreFormat(_)
        | KeyringError::Ambiguous(_) => ErrorKind::Corrupt,
        KeyringError::TooLong(..)
        | KeyringError::NoDefaultStore
        | KeyringError::NotSupportedByStore(_) => ErrorKind::NotAvailable,
        _ => ErrorKind::Backend,
    };
    SecureStorageError::new(kind)
}

/// A store that is not accessible now: locked, or a prompt was refused.
fn no_storage_access(inner: &(dyn std::error::Error + Send + Sync + 'static)) -> ErrorKind {
    #[cfg(target_os = "linux")]
    if let Some(kind) = secret_service_kind(inner) {
        return kind;
    }
    let _ = inner;
    ErrorKind::Locked
}

/// A failed call into the platform store. A missing store is `NotAvailable`, not a retry.
fn platform_failure(inner: &(dyn std::error::Error + Send + Sync + 'static)) -> ErrorKind {
    #[cfg(target_os = "linux")]
    if let Some(kind) = secret_service_kind(inner) {
        return kind;
    }
    let _ = inner;
    ErrorKind::Backend
}

/// Tells the Secret Service's own failures apart: no service, no default collection, a locked
/// keyring, an unlock prompt that did not unlock, and a sandbox or AppArmor denial.
#[cfg(target_os = "linux")]
fn secret_service_kind(
    inner: &(dyn std::error::Error + Send + Sync + 'static),
) -> Option<ErrorKind> {
    use secret_service::Error as Service;
    use zbus::{Error as Zbus, fdo::Error as Fdo};

    Some(match inner.downcast_ref::<Service>()? {
        Service::Locked => ErrorKind::Locked,
        Service::NoResult | Service::Unavailable => ErrorKind::NotAvailable,
        // The unlock prompt ended without an unlock: the keyring is still locked for this call.
        Service::Prompt => ErrorKind::Locked,
        Service::PromptDisconnected => ErrorKind::Backend,
        // No bus at the address the session names: no Secret Service is running here.
        Service::Zbus(Zbus::Connection(..)) => ErrorKind::NotAvailable,
        Service::ZbusFdo(
            Fdo::ServiceUnknown(_) | Fdo::NameHasNoOwner(_) | Fdo::AccessDenied(_),
        ) => ErrorKind::NotAvailable,
        _ => ErrorKind::Backend,
    })
}

/// Turns a name into one that holds only `A-Z a-z 0-9 - _`, so it is safe in a path, a target
/// name or a file. [`decode_name`] reverses it.
pub(crate) fn encode_name(name: &str) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(name.as_bytes())
}

/// Reverses [`encode_name`]. `None` for anything it could not have produced.
pub(crate) fn decode_name(encoded: &str) -> Option<String> {
    let bytes = BASE64_URL_SAFE_NO_PAD.decode(encoded).ok()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::{decode_name, encode_name};

    #[test]
    fn encoded_names_are_safe_and_reverse() {
        for name in ["a/b\\c:d", "..", "ünï 🔑", "", "with spaces and \"quotes\""] {
            let encoded = encode_name(name);
            assert!(
                encoded
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "{encoded:?} has only path-safe characters"
            );
            assert_eq!(decode_name(&encoded).as_deref(), Some(name));
        }
        assert_eq!(decode_name("not base64!"), None);
    }
}
