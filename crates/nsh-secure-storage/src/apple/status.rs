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

#[cfg(test)]
mod tests {
    use super::from_status;
    use crate::error::ErrorKind;

    /// Each Keychain status, with the kind and the code it must map to.
    #[test]
    fn apple_status_mapping_table() {
        let table = [
            (-25308, ErrorKind::Locked, "ERR_SECURE_STORAGE_LOCKED"),
            (
                -34018,
                ErrorKind::MissingEntitlement,
                "ERR_SECURE_STORAGE_MISSING_ENTITLEMENT",
            ),
            (
                -128,
                ErrorKind::AuthCancelled,
                "ERR_SECURE_STORAGE_AUTH_CANCELLED",
            ),
            (
                -25293,
                ErrorKind::AuthFailed,
                "ERR_SECURE_STORAGE_AUTH_FAILED",
            ),
            (-26275, ErrorKind::Corrupt, "ERR_SECURE_STORAGE_CORRUPT"),
            (
                -25291,
                ErrorKind::NotAvailable,
                "ERR_SECURE_STORAGE_NOT_AVAILABLE",
            ),
            (
                -25294,
                ErrorKind::NotAvailable,
                "ERR_SECURE_STORAGE_NOT_AVAILABLE",
            ),
            (-25299, ErrorKind::Backend, "ERR_SECURE_STORAGE_BACKEND"),
        ];
        for (status, kind, code) in table {
            let error = from_status(status);
            assert_eq!(error.kind(), kind, "status {status}");
            assert_eq!(error.code(), code, "status {status}");
            assert_eq!(
                error.status(),
                Some(status),
                "the status is kept on the error"
            );
            assert!(
                !error.message().contains("status"),
                "the message names no number"
            );
        }
    }
}
