//! Options per platform. A store takes defaults; a call may replace them for that call.
//!
//! Each platform reads only its own record. The records follow `flutter_secure_storage`'s
//! names, in snake_case, except where a field is a Rust type.

use std::{path::PathBuf, time::SystemTime};

/// The options a store uses for every call, unless a call gives its own.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecureStorageOptions {
    /// The options for each platform.
    pub platform: PlatformOptions,
}

/// The options of one call. `None` uses the store's options.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallOptions {
    /// Replaces the store's options for this call only.
    pub platform: Option<PlatformOptions>,
}

/// Options of a write, read, lookup, delete or listing. Same shape for each.
pub type WriteOptions = CallOptions;
/// Options of a read. Same shape as [`WriteOptions`].
pub type ReadOptions = CallOptions;
/// Options of a key lookup. Same shape as [`WriteOptions`].
pub type ContainsKeyOptions = CallOptions;
/// Options of a delete. Same shape as [`WriteOptions`].
pub type DeleteOptions = CallOptions;
/// Options of a listing. Same shape as [`WriteOptions`].
pub type ReadAllOptions = CallOptions;
/// Options of a delete-all. Same shape as [`WriteOptions`].
pub type DeleteAllOptions = CallOptions;

/// The options of every platform. A platform reads only its own field.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlatformOptions {
    /// iOS: see [`IosOptions`].
    pub ios: IosOptions,
    /// macOS: see [`MacOsOptions`].
    pub macos: MacOsOptions,
    /// Android: see [`AndroidOptions`].
    pub android: AndroidOptions,
    /// Windows: see [`WindowsOptions`].
    pub windows: WindowsOptions,
    /// Linux: see [`LinuxOptions`].
    pub linux: LinuxOptions,
    /// Web: see [`WebOptions`].
    pub web: WebOptions,
}

/// The keychain's accessibility level for an item (`kSecAttrAccessible`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum KeychainAccessibility {
    /// Only while the device has a passcode and is unlocked. Never migrates to another device.
    Passcode,
    /// Only while the device is unlocked. Migrates to a new device with the backup.
    Unlocked,
    /// Only while unlocked. Never migrates to another device. This is the default.
    #[default]
    UnlockedThisDevice,
    /// After the first unlock since boot.
    FirstUnlock,
    /// After the first unlock since boot. Never migrates to another device.
    FirstUnlockThisDevice,
}

/// A keychain access-control constraint (`SecAccessControlCreateFlags`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessControlFlag {
    /// The device passcode.
    DevicePasscode,
    /// Any enrolled biometry (Touch ID or Face ID).
    BiometryAny,
    /// The biometry enrolled right now. Adding a finger invalidates the item.
    BiometryCurrentSet,
    /// Biometry or the passcode.
    UserPresence,
    /// A paired companion device, such as a watch.
    Watch,
    /// Either of the constraints around it (combine with the others).
    Or,
    /// Both of the constraints around it (combine with the others).
    And,
    /// An application-supplied password.
    ApplicationPassword,
    /// Use of a private key, without a prompt.
    PrivateKeyUsage,
}

/// Whether a biometric or passcode prompt may appear (`kSecUseAuthenticationUI`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthenticationUi {
    /// The system may show the prompt.
    Allow,
    /// Never show a prompt; fail with [`crate::ErrorKind::Locked`] or an auth error instead.
    Fail,
    /// Skip items that need a prompt, and return the rest.
    Skip,
}

/// Options for Apple's keychain, shared by iOS and macOS.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleOptions {
    /// The service name every item is filed under (`kSecAttrService`). `read_all` and
    /// `delete_all` touch only this service.
    pub account_name: String,
    /// The access group items are shared through (`kSecAttrAccessGroup`).
    pub group_id: Option<String>,
    /// When the item can be read (`kSecAttrAccessible`).
    pub accessibility: KeychainAccessibility,
    /// Whether the item syncs through iCloud Keychain (`kSecAttrSynchronizable`).
    pub synchronizable: bool,
    /// Constraints checked on each access (`SecAccessControl`). A non-empty list replaces `accessibility`'s attribute.
    pub access_control_flags: Vec<AccessControlFlag>,
    /// The text shown in an authentication prompt (`kSecUseOperationPrompt`).
    pub operation_prompt: Option<String>,
    /// The item's label (`kSecAttrLabel`).
    pub label: Option<String>,
    /// The item's description (`kSecAttrDescription`).
    pub description: Option<String>,
    /// The item's comment (`kSecAttrComment`).
    pub comment: Option<String>,
    /// Whether the item is hidden from lists (`kSecAttrIsInvisible`).
    pub is_invisible: Option<bool>,
    /// Whether the item is a placeholder (`kSecAttrIsNegative`).
    pub is_negative: Option<bool>,
    /// The most items `read_all` returns. `None` returns all of them.
    pub result_limit: Option<usize>,
    /// Creation time. The keychain sets it, so it is not sent on write.
    pub creation_date: Option<SystemTime>,
    /// Last modification time. The keychain sets it, so it is not sent on write.
    pub last_modified_date: Option<SystemTime>,
    /// Accepted for `flutter_secure_storage` compatibility. Persistent references are not exposed, so it has no effect.
    pub should_return_persistent_reference: Option<bool>,
    /// Whether the system may show authentication prompts (`kSecUseAuthenticationUI`).
    pub authentication_ui: Option<AuthenticationUi>,
    /// Asks for Secure Enclave wrapping. Not built yet: a call with it set fails with `NotAvailable`, never falls back silently.
    pub use_secure_enclave: bool,
}

impl Default for AppleOptions {
    fn default() -> Self {
        Self {
            account_name: "nsh_secure_storage".to_owned(),
            group_id: None,
            accessibility: KeychainAccessibility::default(),
            synchronizable: false,
            access_control_flags: Vec::new(),
            operation_prompt: None,
            label: None,
            description: None,
            comment: None,
            is_invisible: None,
            is_negative: None,
            result_limit: None,
            creation_date: None,
            last_modified_date: None,
            should_return_persistent_reference: None,
            authentication_ui: None,
            use_secure_enclave: false,
        }
    }
}

/// iOS options: the shared Apple options, with nothing iOS-only yet.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IosOptions {
    /// The shared Apple options.
    pub apple: AppleOptions,
}

/// macOS options: the shared Apple options, and where the keychain is.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacOsOptions {
    /// The shared Apple options.
    pub apple: AppleOptions,
    /// Use the data protection keychain (`kSecUseDataProtectionKeychain`). Only this keychain
    /// takes access groups and iCloud sync. When `false`, the file keychain is used.
    pub uses_data_protection_keychain: bool,
    /// The keychain file to use instead of the default one. Tests use a throwaway file here.
    /// It takes effect when `uses_data_protection_keychain` is `false`.
    pub keychain_path: Option<PathBuf>,
}

impl Default for MacOsOptions {
    fn default() -> Self {
        Self {
            apple: AppleOptions::default(),
            uses_data_protection_keychain: true,
            keychain_path: None,
        }
    }
}

/// Android options. The Keystore key is AES-256-GCM. Upstream's cipher choices are not offered:
/// there is one cipher, so there is nothing to choose.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidOptions {
    /// Clears all data when the key is permanently invalidated. Off by default: a silent
    /// reset loses the user's data. Only an explicit, permanent signal can trigger it.
    pub reset_on_error: bool,
    /// Requires a biometric check before each use of the key.
    pub enforce_biometrics: bool,
    /// Keeps the key in StrongBox, the separate secure chip. A device without one returns
    /// `NotAvailable`; nothing falls back silently.
    pub require_strong_box: bool,
    /// Asks for a biometric check on each operation, not once per session.
    pub require_biometrics_per_operation: bool,
    /// Which biometric or credential types the prompt accepts.
    pub biometric_type: AndroidBiometricType,
    /// Whether the prompt needs one more tap (Android 10 and up).
    pub require_biometric_confirmation: bool,
    /// The name space the key and the data live under, to isolate two stores in one app.
    pub storage_namespace: Option<String>,
    /// The biometric prompt's title.
    pub biometric_prompt_title: Option<String>,
    /// The biometric prompt's subtitle.
    pub biometric_prompt_subtitle: Option<String>,
    /// The biometric prompt's cancel button label.
    pub biometric_prompt_negative_button: Option<String>,
}

impl Default for AndroidOptions {
    fn default() -> Self {
        Self {
            reset_on_error: false,
            enforce_biometrics: false,
            require_strong_box: false,
            require_biometrics_per_operation: false,
            biometric_type: AndroidBiometricType::default(),
            require_biometric_confirmation: true,
            storage_namespace: None,
            biometric_prompt_title: None,
            biometric_prompt_subtitle: None,
            biometric_prompt_negative_button: None,
        }
    }
}

/// Which credentials an Android biometric prompt accepts.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AndroidBiometricType {
    /// Strong biometrics only. Device credentials are rejected.
    StrongBiometricOnly,
    /// Strong biometrics or the device credential. This is the default.
    #[default]
    BiometricOrDeviceCredential,
}

/// Windows options. Each app's values and Credential Manager key are its own: the app identity
/// names both, so two apps never read each other's values.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsOptions {
    /// The app's identity. `None` reads the app id NSH sets for the process (its
    /// AppUserModelID, which is `nsh.json`'s `id`). Set it when the process has none.
    pub app_id: Option<String>,
    /// The store's name. Stores with different names never share a value.
    pub account_name: String,
}

impl Default for WindowsOptions {
    fn default() -> Self {
        Self {
            app_id: None,
            account_name: "nsh_secure_storage".to_owned(),
        }
    }
}

/// Linux options. The store is one Secret Service namespace per user, so apps on one desktop
/// account that keep the default name share it: give each app its own name.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxOptions {
    /// The store's name: the Secret Service `service` attribute every item carries.
    pub account_name: String,
}

impl Default for LinuxOptions {
    fn default() -> Self {
        Self {
            account_name: "nsh_secure_storage".to_owned(),
        }
    }
}

/// Web options. The web has no secret store: the key lives in the page's own IndexedDB, so a
/// script on the page can use it (it cannot export it). See the "Web backend" section of `docs/decisions.md`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebOptions {
    /// The IndexedDB database the data lives in. It is also the store's name: stores with
    /// different names are isolated, and `delete_all` touches only this one.
    pub db_name: String,
    /// Upstream's `useSessionStorage`, for data that lasts only as long as the tab. Not supported:
    /// a key in IndexedDB outlives the tab. When `true`, every call fails with
    /// `ERR_SECURE_STORAGE_NOT_AVAILABLE`, and nothing is stored under another mode.
    pub use_session_storage: bool,
}

impl Default for WebOptions {
    fn default() -> Self {
        Self {
            db_name: "nsh_secure_storage".to_owned(),
            use_session_storage: false,
        }
    }
}
