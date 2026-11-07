//! Security.framework constants `security-framework-sys` does not export, and the OSStatus codes
//! this backend maps. Each value is from the SDK's `SecBase.h`.

use core_foundation_sys::base::OSStatus;
use core_foundation::string::CFStringRef;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    /// `SecKeychainGetStatus`: reports the keychain's state bits. Deprecated, but the file keychain has no other call for it.
    pub fn SecKeychainGetStatus(keychain: *const std::ffi::c_void, status: *mut u32) -> OSStatus;
    pub static kSecAttrAccessible: CFStringRef;
    pub static kSecAttrIsInvisible: CFStringRef;
    pub static kSecAttrIsNegative: CFStringRef;
    pub static kSecMatchLimitOne: CFStringRef;
    pub static kSecUseOperationPrompt: CFStringRef;
    pub static kSecUseDataProtectionKeychain: CFStringRef;
    pub static kSecUseAuthenticationUIAllow: CFStringRef;
    pub static kSecUseAuthenticationUIFail: CFStringRef;
}

/// `errSecUserCanceled`.
pub const ERR_SEC_USER_CANCELED: OSStatus = -128;
/// `errSecNotAvailable`.
pub const ERR_SEC_NOT_AVAILABLE: OSStatus = -25291;
/// `errSecAuthFailed`.
pub const ERR_SEC_AUTH_FAILED: OSStatus = -25293;
/// `errSecNoSuchKeychain`.
pub const ERR_SEC_NO_SUCH_KEYCHAIN: OSStatus = -25294;
/// `errSecDuplicateItem`.
pub const ERR_SEC_DUPLICATE_ITEM: OSStatus = -25299;
/// `errSecItemNotFound`.
pub const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;
/// `errSecInteractionNotAllowed`.
pub const ERR_SEC_INTERACTION_NOT_ALLOWED: OSStatus = -25308;
/// `errSecDecode`.
pub const ERR_SEC_DECODE: OSStatus = -26275;
/// `errSecMissingEntitlement`.
pub const ERR_SEC_MISSING_ENTITLEMENT: OSStatus = -34018;

/// `kSecUnlockStateStatus`: set while the keychain is unlocked.
pub const K_SEC_UNLOCK_STATE_STATUS: u32 = 1;
