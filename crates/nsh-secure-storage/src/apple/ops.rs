//! The Keychain operations. A call's [`Spec`] is plain owned data, so it moves to a blocking
//! thread; Core Foundation objects are made and released on that thread.

use std::{collections::BTreeMap, path::PathBuf, ptr};

use core_foundation::{
    array::CFArray,
    base::{CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    data::CFData,
    dictionary::{CFDictionary, CFMutableDictionary},
    number::CFNumber,
    string::{CFString, CFStringRef},
};
use security_framework::access_control::{ProtectionMode, SecAccessControl};
use core_foundation_sys::base::OSStatus;
use security_framework_sys::{
    access_control::{
        kSecAttrAccessibleAfterFirstUnlock, kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
        kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly, kSecAttrAccessibleWhenUnlocked,
        kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
    },
    item::{
        kSecAttrAccessControl, kSecAttrAccessGroup, kSecAttrAccount, kSecAttrComment,
        kSecAttrDescription, kSecAttrLabel, kSecAttrService, kSecAttrSynchronizable,
        kSecAttrSynchronizableAny, kSecClass, kSecClassGenericPassword, kSecMatchLimit,
        kSecMatchLimitAll, kSecMatchSearchList, kSecReturnAttributes, kSecReturnData,
        kSecUseAuthenticationUI,
        kSecUseAuthenticationUISkip, kSecUseKeychain, kSecValueData,
    },
    keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemDelete, SecItemUpdate},
};
use zeroize::Zeroizing;

use super::{
    status::from_status,
    sys::{
        ERR_SEC_DUPLICATE_ITEM, ERR_SEC_ITEM_NOT_FOUND, kSecAttrAccessible, kSecAttrIsInvisible,
        kSecAttrIsNegative, kSecMatchLimitOne, kSecUseAuthenticationUIAllow,
        kSecUseAuthenticationUIFail, kSecUseDataProtectionKeychain, kSecUseOperationPrompt,
    },
};
use crate::{
    error::{ErrorKind, SecureStorageError},
    options::{AccessControlFlag, AuthenticationUi, KeychainAccessibility, PlatformOptions},
};

/// Reads one of Security's constants. They are valid for the whole process once the framework loads.
macro_rules! sec {
    ($constant:ident) => {
        // SAFETY: Security's constants are initialized by the framework before this code runs.
        unsafe { $constant }
    };
}

/// The Keychain dictionary type every query and attribute set uses.
type Dict = CFMutableDictionary<CFString, CFType>;

/// What one call addresses: a service of the Keychain, and how to reach it. Owned data only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spec {
    /// The service every item is filed under (`sec!(kSecAttrService)`).
    service: String,
    /// The access group items are shared through.
    group: Option<String>,
    /// A keychain file to use instead of the default one (macOS, file keychain only).
    keychain_file: Option<PathBuf>,
    /// Whether the data protection keychain is used. It alone takes iCloud sync in every case.
    data_protection: bool,
    synchronizable: bool,
    accessibility: KeychainAccessibility,
    flags: Vec<AccessControlFlag>,
    auth_ui: Option<AuthenticationUi>,
    operation_prompt: Option<String>,
    label: Option<String>,
    description: Option<String>,
    comment: Option<String>,
    is_invisible: Option<bool>,
    is_negative: Option<bool>,
    result_limit: Option<usize>,
}

impl Spec {
    /// The spec of the platform's options. Options this backend does not build yet are refused.
    pub(crate) fn from_options(options: &PlatformOptions) -> Result<Self, SecureStorageError> {
        #[cfg(target_os = "ios")]
        let (apple, keychain_file, data_protection) = (&options.ios.apple, None, true);
        #[cfg(target_os = "macos")]
        let (apple, keychain_file, data_protection) = {
            let mac = &options.macos;
            let file = if mac.uses_data_protection_keychain {
                None
            } else {
                mac.keychain_path.clone()
            };
            (&mac.apple, file, mac.uses_data_protection_keychain)
        };
        if apple.use_secure_enclave {
            return Err(SecureStorageError::new(ErrorKind::NotAvailable));
        }
        Ok(Self {
            service: apple.account_name.clone(),
            group: apple.group_id.clone(),
            keychain_file,
            data_protection,
            synchronizable: apple.synchronizable,
            accessibility: apple.accessibility,
            flags: apple.access_control_flags.clone(),
            auth_ui: apple.authentication_ui,
            operation_prompt: apple.operation_prompt.clone(),
            label: apple.label.clone(),
            description: apple.description.clone(),
            comment: apple.comment.clone(),
            is_invisible: apple.is_invisible,
            is_negative: apple.is_negative,
            result_limit: apple.result_limit,
        })
    }

    /// Names the store: the items a service holds. Accessibility and flags do not name it, since
    /// reads find an item at any accessibility level.
    pub(crate) fn identity(&self) -> String {
        format!(
            "apple|{}|{:?}|{:?}|{}",
            self.service, self.group, self.keychain_file, self.data_protection
        )
    }

    /// Whether searches match both synchronizable and local items. The file keychain takes no such attribute.
    fn matches_any_sync(&self) -> bool {
        self.data_protection
    }
}

/// Holds the no-prompt guard for a call whose spec asks for no authentication prompt.
fn no_prompt_guard(spec: &Spec) -> Option<super::prompt::NoPrompt> {
    #[cfg(target_os = "macos")]
    {
        match spec.auth_ui {
            Some(AuthenticationUi::Fail | AuthenticationUi::Skip) => Some(super::prompt::NoPrompt::enter()),
            _ => None,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = spec;
        None
    }
}

/// Writes `value` under `key`. Updates an item in place when there is one, and adds it otherwise.
/// A failed update never removes the item.
pub(crate) fn write(spec: &Spec, key: &str, value: &str) -> Result<(), SecureStorageError> {
    let _no_prompt = no_prompt_guard(spec);
    let keychain = open_keychain(spec)?;
    let data = CFData::from_buffer(value.as_bytes());

    let search = search_query(spec, keychain.as_ref(), key);
    let mut update = Dict::new();
    put(&mut update, sec!(kSecValueData), data.clone().into_CFType());
    put_attributes(spec, &mut update, Mode::Update)?;
    match update_item(&search, &update) {
        0 => return Ok(()),
        ERR_SEC_ITEM_NOT_FOUND => {}
        status => return Err(from_status(status)),
    }

    let mut add = scope(spec, keychain.as_ref(), Purpose::Add);
    put_str(&mut add, sec!(kSecAttrAccount), key);
    put(&mut add, sec!(kSecValueData), data.into_CFType());
    put_attributes(spec, &mut add, Mode::Add)?;
    if spec.data_protection || spec.synchronizable {
        put(&mut add, sec!(kSecAttrSynchronizable), bool_value(spec.synchronizable));
    }
    match add_item(&add) {
        0 => Ok(()),
        ERR_SEC_DUPLICATE_ITEM => Err(from_status(ERR_SEC_DUPLICATE_ITEM).with_key(key)),
        status => Err(from_status(status)),
    }
}

/// The value under `key`, or `None` when there is none. A value that is not UTF-8 is `Corrupt`.
pub(crate) fn read(
    spec: &Spec,
    key: &str,
) -> Result<Option<Zeroizing<String>>, SecureStorageError> {
    let _no_prompt = no_prompt_guard(spec);
    let keychain = open_keychain(spec)?;
    let mut query = search_query(spec, keychain.as_ref(), key);
    put(&mut query, sec!(kSecReturnData), bool_value(true));
    put(&mut query, sec!(kSecMatchLimit), constant(sec!(kSecMatchLimitOne)));
    let mut found: CFTypeRef = ptr::null();
    match copy_matching(&query, Some(&mut found)) {
        0 => {}
        ERR_SEC_ITEM_NOT_FOUND => return Ok(None),
        status => return Err(from_status(status)),
    }
    // SAFETY: a successful SecItemCopyMatching returns a retained reference the caller owns.
    let found = unsafe { CFType::wrap_under_create_rule(found) };
    let data = found
        .downcast_into::<CFData>()
        .ok_or_else(|| SecureStorageError::new(ErrorKind::Corrupt).with_key(key))?;
    decode(data.bytes(), key).map(Some)
}

/// Whether an item exists under `key`. It reads no data, so no value is fetched.
pub(crate) fn contains_key(spec: &Spec, key: &str) -> Result<bool, SecureStorageError> {
    let _no_prompt = no_prompt_guard(spec);
    let keychain = open_keychain(spec)?;
    let mut query = search_query(spec, keychain.as_ref(), key);
    put(&mut query, sec!(kSecMatchLimit), constant(sec!(kSecMatchLimitOne)));
    match copy_matching(&query, None) {
        0 => Ok(true),
        ERR_SEC_ITEM_NOT_FOUND => Ok(false),
        status => Err(from_status(status)),
    }
}

/// Removes the item under `key`. Nothing happens when there is none.
pub(crate) fn delete(spec: &Spec, key: &str) -> Result<(), SecureStorageError> {
    let _no_prompt = no_prompt_guard(spec);
    let keychain = open_keychain(spec)?;
    let query = search_query(spec, keychain.as_ref(), key);
    match delete_item(&query) {
        0 | ERR_SEC_ITEM_NOT_FOUND => Ok(()),
        status => Err(from_status(status)),
    }
}

/// Every item of the service, in key order, at most `result_limit` of them.
pub(crate) fn read_all(
    spec: &Spec,
) -> Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError> {
    let _no_prompt = no_prompt_guard(spec);
    let keychain = open_keychain(spec)?;
    let mut query = scope(spec, keychain.as_ref(), Purpose::Search);
    apply_authentication(spec, &mut query);
    put(&mut query, sec!(kSecReturnAttributes), bool_value(true));
    put(&mut query, sec!(kSecReturnData), bool_value(true));
    let limit = match spec.result_limit {
        Some(n) => CFNumber::from(i64::try_from(n).unwrap_or(i64::MAX)).into_CFType(),
        None => constant(sec!(kSecMatchLimitAll)),
    };
    put(&mut query, sec!(kSecMatchLimit), limit);

    let mut found: CFTypeRef = ptr::null();
    match copy_matching(&query, Some(&mut found)) {
        0 => {}
        ERR_SEC_ITEM_NOT_FOUND => return Ok(BTreeMap::new()),
        status => return Err(from_status(status)),
    }
    // SAFETY: a successful SecItemCopyMatching returns a retained reference the caller owns.
    let found = unsafe { CFType::wrap_under_create_rule(found) };
    let items = found
        .downcast_into::<CFArray>()
        .ok_or_else(|| SecureStorageError::new(ErrorKind::Backend))?;
    // SAFETY: the array is the one just checked to be a CFArray, and it outlives this borrow.
    let items: CFArray<CFType> = unsafe { CFArray::wrap_under_get_rule(items.as_concrete_TypeRef()) };

    let mut all = BTreeMap::new();
    for item in items.iter() {
        let Some(item) = item.downcast::<CFDictionary>() else {
            continue;
        };
        // SAFETY: the dictionary is the one just checked to be a CFDictionary.
        let item: CFDictionary<CFType, CFType> =
            unsafe { CFDictionary::wrap_under_get_rule(item.as_concrete_TypeRef()) };
        let account = item
            .find(&constant(sec!(kSecAttrAccount)))
            .and_then(|account| account.downcast::<CFString>())
            .map(|account| account.to_string());
        let data = item
            .find(&constant(sec!(kSecValueData)))
            .and_then(|data| data.downcast::<CFData>());
        let (Some(account), Some(data)) = (account, data) else {
            continue;
        };
        let value = decode(data.bytes(), &account)?;
        all.insert(account, value);
    }
    Ok(all)
}

/// Removes every item of the service, and nothing of any other service.
pub(crate) fn delete_all(spec: &Spec) -> Result<(), SecureStorageError> {
    let _no_prompt = no_prompt_guard(spec);
    let keychain = open_keychain(spec)?;
    let mut query = scope(spec, keychain.as_ref(), Purpose::Search);
    apply_authentication(spec, &mut query);
    match delete_item(&query) {
        0 | ERR_SEC_ITEM_NOT_FOUND => Ok(()),
        status => Err(from_status(status)),
    }
}

/// How an attribute set is used: `Add` writes every attribute, `Update` only those that may change.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Add,
    Update,
}

/// What a dictionary is for: a search names its keychain in the search list, an add in `kSecUseKeychain`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Search,
    Add,
}

/// The query that finds every item of the service (or the attributes of an add), in `keychain` when given.
fn scope(spec: &Spec, keychain: Option<&SecKeychainHandle>, purpose: Purpose) -> Dict {
    let mut dict = Dict::new();
    put(&mut dict, sec!(kSecClass), constant(sec!(kSecClassGenericPassword)));
    put_str(&mut dict, sec!(kSecAttrService), &spec.service);
    if let Some(group) = &spec.group {
        put_str(&mut dict, sec!(kSecAttrAccessGroup), group);
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(keychain) = keychain {
            // SAFETY: the keychain reference is alive for the call, and CFType retains it.
            let reference = unsafe { CFType::wrap_under_get_rule(keychain.as_CFTypeRef()) };
            match purpose {
                Purpose::Search => put(
                    &mut dict,
                    sec!(kSecMatchSearchList),
                    CFArray::from_CFTypes(&[reference]).into_CFType(),
                ),
                Purpose::Add => put(&mut dict, sec!(kSecUseKeychain), reference),
            }
        }
        if spec.data_protection {
            put(&mut dict, sec!(kSecUseDataProtectionKeychain), bool_value(true));
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (keychain, purpose);
    if spec.matches_any_sync() {
        put(&mut dict, sec!(kSecAttrSynchronizable), constant(sec!(kSecAttrSynchronizableAny)));
    }
    dict
}

/// The query for one item: the scope, its account, and the authentication behavior.
fn search_query(spec: &Spec, keychain: Option<&SecKeychainHandle>, key: &str) -> Dict {
    let mut dict = scope(spec, keychain, Purpose::Search);
    put_str(&mut dict, sec!(kSecAttrAccount), key);
    apply_authentication(spec, &mut dict);
    dict
}

/// Adds the prompt and authentication behavior a query asks for.
fn apply_authentication(spec: &Spec, dict: &mut Dict) {
    if let Some(ui) = spec.auth_ui {
        let value = match ui {
            AuthenticationUi::Allow => sec!(kSecUseAuthenticationUIAllow),
            AuthenticationUi::Fail => sec!(kSecUseAuthenticationUIFail),
            AuthenticationUi::Skip => sec!(kSecUseAuthenticationUISkip),
        };
        put(dict, sec!(kSecUseAuthenticationUI), constant(value));
    }
    if let Some(prompt) = &spec.operation_prompt {
        put_str(dict, sec!(kSecUseOperationPrompt), prompt);
    }
}

/// Adds the item's attributes. An access-control constraint replaces the accessibility attribute.
fn put_attributes(spec: &Spec, dict: &mut Dict, mode: Mode) -> Result<(), SecureStorageError> {
    if let Some(label) = &spec.label {
        put_str(dict, sec!(kSecAttrLabel), label);
    }
    if let Some(description) = &spec.description {
        put_str(dict, sec!(kSecAttrDescription), description);
    }
    if let Some(comment) = &spec.comment {
        put_str(dict, sec!(kSecAttrComment), comment);
    }
    if let Some(invisible) = spec.is_invisible {
        put(dict, sec!(kSecAttrIsInvisible), bool_value(invisible));
    }
    if let Some(negative) = spec.is_negative {
        put(dict, sec!(kSecAttrIsNegative), bool_value(negative));
    }
    if spec.flags.is_empty() {
        put(dict, sec!(kSecAttrAccessible), constant(accessible_value(spec.accessibility)));
    } else if mode == Mode::Add {
        let protection = protection_mode(spec.accessibility);
        let control = SecAccessControl::create_with_protection(Some(protection), flag_bits(&spec.flags))
            .map_err(|error| from_status(error.code()))?;
        put(dict, sec!(kSecAttrAccessControl), control.into_CFType());
    }
    Ok(())
}

/// The `sec!(kSecAttrAccessible)` value of a level.
fn accessible_value(level: KeychainAccessibility) -> CFStringRef {
    match level {
        KeychainAccessibility::Passcode => sec!(kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly),
        KeychainAccessibility::Unlocked => sec!(kSecAttrAccessibleWhenUnlocked),
        KeychainAccessibility::UnlockedThisDevice => sec!(kSecAttrAccessibleWhenUnlockedThisDeviceOnly),
        KeychainAccessibility::FirstUnlock => sec!(kSecAttrAccessibleAfterFirstUnlock),
        KeychainAccessibility::FirstUnlockThisDevice => {
            sec!(kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly)
        }
    }
}

/// The same level as the protection an access control is created with.
fn protection_mode(level: KeychainAccessibility) -> ProtectionMode {
    match level {
        KeychainAccessibility::Passcode => ProtectionMode::AccessibleWhenPasscodeSetThisDeviceOnly,
        KeychainAccessibility::Unlocked => ProtectionMode::AccessibleWhenUnlocked,
        KeychainAccessibility::UnlockedThisDevice => {
            ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly
        }
        KeychainAccessibility::FirstUnlock => ProtectionMode::AccessibleAfterFirstUnlock,
        KeychainAccessibility::FirstUnlockThisDevice => {
            ProtectionMode::AccessibleAfterFirstUnlockThisDeviceOnly
        }
    }
}

/// `SecAccessControlCreateFlags` for a set of flags, as the SDK's `SecAccessControl.h` numbers them.
fn flag_bits(flags: &[AccessControlFlag]) -> usize {
    flags.iter().fold(0_usize, |bits, flag| {
        bits | match flag {
            AccessControlFlag::UserPresence => 1_usize << 0,
            AccessControlFlag::BiometryAny => 1_usize << 1,
            AccessControlFlag::BiometryCurrentSet => 1_usize << 3,
            AccessControlFlag::DevicePasscode => 1_usize << 4,
            AccessControlFlag::Watch => 1_usize << 5,
            AccessControlFlag::Or => 1_usize << 14,
            AccessControlFlag::And => 1_usize << 15,
            AccessControlFlag::PrivateKeyUsage => 1_usize << 30,
            AccessControlFlag::ApplicationPassword => 1_usize << 31,
        }
    })
}

/// Reads stored bytes as UTF-8. The copy is zeroed when dropped.
fn decode(bytes: &[u8], key: &str) -> Result<Zeroizing<String>, SecureStorageError> {
    std::str::from_utf8(bytes)
        .map(|text| Zeroizing::new(text.to_owned()))
        .map_err(|_| SecureStorageError::new(ErrorKind::Corrupt).with_key(key))
}

/// The keychain file a call uses, on macOS. Other platforms have none.
#[cfg(target_os = "macos")]
type SecKeychainHandle = security_framework::os::macos::keychain::SecKeychain;
#[cfg(not(target_os = "macos"))]
type SecKeychainHandle = ();

/// Opens the keychain file the spec names, on macOS. Elsewhere there is no file keychain.
#[cfg(target_os = "macos")]
fn open_keychain(spec: &Spec) -> Result<Option<SecKeychainHandle>, SecureStorageError> {
    match &spec.keychain_file {
        Some(path) => SecKeychainHandle::open(path)
            .map(Some)
            .map_err(|error| from_status(error.code())),
        None => Ok(None),
    }
}

#[cfg(not(target_os = "macos"))]
fn open_keychain(_spec: &Spec) -> Result<Option<SecKeychainHandle>, SecureStorageError> {
    Ok(None)
}

/// A `CFType` for a Security constant.
fn constant(value: CFStringRef) -> CFType {
    // SAFETY: Security's constants are valid for the process, and CFType retains them.
    unsafe { CFType::wrap_under_get_rule(value as CFTypeRef) }
}

fn bool_value(value: bool) -> CFType {
    CFBoolean::from(value).into_CFType()
}

fn put(dict: &mut Dict, key: CFStringRef, value: CFType) {
    // SAFETY: the key is one of Security's constants, valid for the process.
    dict.set(unsafe { CFString::wrap_under_get_rule(key) }, value);
}

fn put_str(dict: &mut Dict, key: CFStringRef, value: &str) {
    put(dict, key, CFString::new(value).into_CFType());
}

/// `SecItemCopyMatching`, asking for a result only when `result` is given.
fn copy_matching(query: &Dict, result: Option<&mut CFTypeRef>) -> OSStatus {
    let out = result.map_or(ptr::null_mut(), |result| result as *mut CFTypeRef);
    // SAFETY: the query is a live dictionary; `out` is null or a live slot for the result.
    unsafe { SecItemCopyMatching(query.as_concrete_TypeRef() as _, out) }
}

fn update_item(search: &Dict, update: &Dict) -> OSStatus {
    // SAFETY: both dictionaries are live for the call.
    unsafe {
        SecItemUpdate(
            search.as_concrete_TypeRef() as _,
            update.as_concrete_TypeRef() as _,
        )
    }
}

fn add_item(attributes: &Dict) -> OSStatus {
    // SAFETY: the dictionary is live for the call, and no result is requested.
    unsafe { SecItemAdd(attributes.as_concrete_TypeRef() as _, ptr::null_mut()) }
}

fn delete_item(query: &Dict) -> OSStatus {
    // SAFETY: the query is a live dictionary for the call.
    unsafe { SecItemDelete(query.as_concrete_TypeRef() as _) }
}
