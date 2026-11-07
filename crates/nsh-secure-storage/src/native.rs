//! The store as TSX sees it: `NativeSecureStorage`, a class whose async members reject with a
//! `code` on failure. Values reach JavaScript only through `read` and the listener, which is the
//! point of the store; every other member returns no value.

use nsh_core::prelude::*;

use crate::{
    AccessControlFlag, AuthenticationUi, KeychainAccessibility, SecureStorage, SecureStorageError,
    SecureStorageOptions,
};

/// The options of a store, from TSX. Unset fields keep the defaults.
#[derive(JsObject, Clone, Debug, Default)]
pub struct StoreOptions {
    /// The service the items are filed under (Apple). Default `nsh_secure_storage`.
    pub account_name: Option<String>,
    /// The access group items are shared through (Apple).
    pub group_id: Option<String>,
    /// `"passcode"`, `"unlocked"`, `"unlockedThisDevice"`, `"firstUnlock"` or `"firstUnlockThisDevice"`.
    pub accessibility: Option<String>,
    /// Whether the item syncs through iCloud Keychain (Apple).
    pub synchronizable: Option<bool>,
    /// Access-control flags such as `"userPresence"` or `"biometryCurrentSet"`. See the Rust docs.
    pub access_control_flags: Option<Vec<String>>,
    /// `"allow"`, `"fail"` or `"skip"`. `"fail"` and `"skip"` never show a system dialog.
    pub authentication_ui: Option<String>,
    /// The text shown in an authentication prompt (Apple).
    pub operation_prompt: Option<String>,
    /// Whether macOS uses the data protection keychain. Default `true`.
    pub uses_data_protection_keychain: Option<bool>,
    /// A keychain file to use on macOS, when the data protection keychain is off.
    pub keychain_path: Option<String>,
}

/// One item of `readAll`.
#[derive(JsObject, Clone, Debug, PartialEq, Eq)]
pub struct StoredEntry {
    /// The key.
    pub key: String,
    /// The value.
    pub value: String,
}

/// A secure store. Create one per store and keep it.
#[class]
pub struct NativeSecureStorage {
    store: SecureStorage,
}

#[class]
impl NativeSecureStorage {
    /// A store on the platform's secret store. `options` sets its defaults.
    // Refuses an unknown option value with a TypeError that names it.
    pub fn new(options: Option<StoreOptions>) -> Result<NativeSecureStorage, String> {
        let options = match options {
            Some(options) => to_store_options(&options)?,
            None => SecureStorageOptions::default(),
        };
        Ok(NativeSecureStorage {
            store: SecureStorage::new(options),
        })
    }

    /// An empty store in memory: for tests, and for apps that keep nothing on the device.
    pub fn memory() -> NativeSecureStorage {
        NativeSecureStorage {
            store: SecureStorage::memory(),
        }
    }

    /// Stores `value` under `key`. A `null` value deletes the key.
    pub async fn write(
        this: Instance<Self>,
        key: String,
        value: Option<String>,
    ) -> Result<(), JsError> {
        let store = this.with(|native| native.store);
        store
            .write(&key, value.as_deref(), &Default::default())
            .await
            .map_err(into_js)
    }

    /// The value under `key`, or `null` when there is none.
    pub async fn read(this: Instance<Self>, key: String) -> Result<Option<String>, JsError> {
        let store = this.with(|native| native.store);
        let value = store
            .read(&key, &Default::default())
            .await
            .map_err(into_js)?;
        Ok(value.map(|value| value.to_string()))
    }

    /// Whether a value is stored under `key`.
    pub async fn contains_key(this: Instance<Self>, key: String) -> Result<bool, JsError> {
        let store = this.with(|native| native.store);
        store
            .contains_key(&key, &Default::default())
            .await
            .map_err(into_js)
    }

    /// Removes the value under `key`. A key that is not there is not an error.
    pub async fn delete(this: Instance<Self>, key: String) -> Result<(), JsError> {
        let store = this.with(|native| native.store);
        store
            .delete(&key, &Default::default())
            .await
            .map_err(into_js)
    }

    /// Every key and value of the store, in key order.
    pub async fn read_all(this: Instance<Self>) -> Result<Vec<StoredEntry>, JsError> {
        let store = this.with(|native| native.store);
        let all = store.read_all(&Default::default()).await.map_err(into_js)?;
        Ok(all
            .into_iter()
            .map(|(key, value)| StoredEntry {
                key,
                value: value.to_string(),
            })
            .collect())
    }

    /// Removes every value of the store.
    pub async fn delete_all(this: Instance<Self>) -> Result<(), JsError> {
        let store = this.with(|native| native.store);
        store.delete_all(&Default::default()).await.map_err(into_js)
    }

    /// Whether the platform's protected data is readable now. Always `true` off iOS.
    pub fn is_protected_data_available(&self) -> bool {
        self.store.is_protected_data_available()
    }

    /// Calls `listener` with the new value of `key`, or `null`, after each write or delete made
    /// through this store. The returned function ends it.
    pub fn on_change(
        &self,
        key: String,
        #[args(value)] listener: Callback<Option<String>>,
    ) -> Stop {
        let subscription = self.store.register_listener(&key, move |value| {
            listener.post(value.map(str::to_owned));
        });
        Stop::new(move || subscription.stop())
    }
}

/// The error a rejection carries: its message, and the `code` apps match on.
fn into_js(error: SecureStorageError) -> JsError {
    let message = match error.key() {
        Some(key) => format!("{} (key {key:?})", error.message()),
        None => error.message().to_owned(),
    };
    JsError::Error(message).with_code(error.code())
}

/// Turns the TSX options into the store's options, refusing a value it does not know.
fn to_store_options(options: &StoreOptions) -> Result<SecureStorageOptions, String> {
    let mut store = SecureStorageOptions::default();
    let apple = &mut store.platform.macos.apple;
    if let Some(name) = &options.account_name {
        apple.account_name = name.clone();
    }
    apple.group_id = options.group_id.clone();
    apple.operation_prompt = options.operation_prompt.clone();
    if let Some(synchronizable) = options.synchronizable {
        apple.synchronizable = synchronizable;
    }
    if let Some(level) = &options.accessibility {
        apple.accessibility = match level.as_str() {
            "passcode" => KeychainAccessibility::Passcode,
            "unlocked" => KeychainAccessibility::Unlocked,
            "unlockedThisDevice" => KeychainAccessibility::UnlockedThisDevice,
            "firstUnlock" => KeychainAccessibility::FirstUnlock,
            "firstUnlockThisDevice" => KeychainAccessibility::FirstUnlockThisDevice,
            other => return Err(format!("accessibility: unknown level {other:?}")),
        };
    }
    if let Some(flags) = &options.access_control_flags {
        apple.access_control_flags = flags
            .iter()
            .map(|flag| match flag.as_str() {
                "devicePasscode" => Ok(AccessControlFlag::DevicePasscode),
                "biometryAny" => Ok(AccessControlFlag::BiometryAny),
                "biometryCurrentSet" => Ok(AccessControlFlag::BiometryCurrentSet),
                "userPresence" => Ok(AccessControlFlag::UserPresence),
                "watch" => Ok(AccessControlFlag::Watch),
                "or" => Ok(AccessControlFlag::Or),
                "and" => Ok(AccessControlFlag::And),
                "applicationPassword" => Ok(AccessControlFlag::ApplicationPassword),
                "privateKeyUsage" => Ok(AccessControlFlag::PrivateKeyUsage),
                other => Err(format!("accessControlFlags: unknown flag {other:?}")),
            })
            .collect::<Result<_, _>>()?;
    }
    if let Some(ui) = &options.authentication_ui {
        apple.authentication_ui = Some(match ui.as_str() {
            "allow" => AuthenticationUi::Allow,
            "fail" => AuthenticationUi::Fail,
            "skip" => AuthenticationUi::Skip,
            other => return Err(format!("authenticationUI: unknown value {other:?}")),
        });
    }
    let mac = &mut store.platform.macos;
    if let Some(uses) = options.uses_data_protection_keychain {
        mac.uses_data_protection_keychain = uses;
    }
    mac.keychain_path = options.keychain_path.as_ref().map(Into::into);
    Ok(store)
}
