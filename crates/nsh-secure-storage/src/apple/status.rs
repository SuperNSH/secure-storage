//! Maps the Keychain's OSStatus codes to the crate's error kinds.

use core_foundation_sys::base::OSStatus;

use super::sys::{
    ERR_SEC_AUTH_FAILED, ERR_SEC_DECODE, ERR_SEC_INTERACTION_NOT_ALLOWED,
    ERR_SEC_MISSING_ENTITLEMENT, ERR_SEC_NO_SUCH_KEYCHAIN, ERR_SEC_NOT_AVAILABLE,
    ERR_SEC_USER_CANCELED,
};
use crate::error::{ErrorKind, SecureStorageError};

/// The error for a Keychain call that returned `status`, with the status kept on it.
pub(crate) fn from_status(status: OSStatus) -> SecureStorageError {
    let kind = match status {
        ERR_SEC_INTERACTION_NOT_ALLOWED => ErrorKind::Locked,
        ERR_SEC_MISSING_ENTITLEMENT => ErrorKind::MissingEntitlement,
        ERR_SEC_USER_CANCELED => ErrorKind::AuthCancelled,
        ERR_SEC_AUTH_FAILED => ErrorKind::AuthFailed,
        ERR_SEC_DECODE => ErrorKind::Corrupt,
        ERR_SEC_NOT_AVAILABLE | ERR_SEC_NO_SUCH_KEYCHAIN => ErrorKind::NotAvailable,
        _ => ErrorKind::Backend,
    };
    SecureStorageError::new(kind).with_status(status)
}
