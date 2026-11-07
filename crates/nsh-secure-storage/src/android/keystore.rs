//! The seam between the store logic and the platform: the AndroidKeyStore keys, the app's
//! no-backup folder, and the options that shape both. The JNI version is `jni.rs`; the tests use
//! a fake that seals with AES-GCM.

use std::path::PathBuf;

use zeroize::Zeroizing;

use crate::{
    error::{ErrorKind, SecureStorageError},
    options::{AndroidBiometricType, PlatformOptions},
};

/// The namespace used when the options name none.
const DEFAULT_NAMESPACE: &str = "nsh_secure_storage";

/// What a Keystore call can fail with. Only [`VaultError::Invalidated`] and [`VaultError::Tag`]
/// are permanent, and only those may lead to a reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VaultError {
    /// The key was permanently invalidated (`KeyPermanentlyInvalidatedException`), for example
    /// by a change of enrolled biometrics. It can never be used again.
    Invalidated,
    /// The entry does not match its key, its store or its key name (`AEADBadTagException`).
    Tag,
    /// The keystore is locked, or the key needs a fresh authentication (`UserNotAuthenticatedException`).
    /// Retrying after unlock may succeed.
    Locked,
    /// Any other failure, including a platform class that does not load. Never a reason to
    /// replace a key.
    Failed,
}

impl VaultError {
    /// The store error this maps to. Nothing in it names a value.
    pub(crate) fn error(self) -> SecureStorageError {
        let kind = match self {
            Self::Invalidated => ErrorKind::KeyInvalidated,
            Self::Tag => ErrorKind::Corrupt,
            Self::Locked => ErrorKind::Locked,
            Self::Failed => ErrorKind::Backend,
        };
        SecureStorageError::new(kind)
    }
}

/// One sealed value: the Keystore's random IV, and the ciphertext with its tag.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Sealed {
    pub(crate) iv: Vec<u8>,
    pub(crate) data: Vec<u8>,
}

/// The platform side of a store. Every method may block; the backend runs them on a blocking thread.
pub(crate) trait Keystore: Send + Sync {
    /// The app's folder that backup and device transfer leave out.
    fn no_backup_dir(&self) -> Result<PathBuf, VaultError>;

    /// Whether the store's key exists. An error is not an answer: it never means "absent".
    fn has_key(&self, policy: &Policy) -> Result<bool, VaultError>;

    /// Makes the store's key. Only called when [`Keystore::has_key`] said there is none.
    fn create_key(&self, policy: &Policy) -> Result<(), VaultError>;

    /// Removes the store's key. Nothing happens when there is none.
    fn delete_key(&self, policy: &Policy) -> Result<(), VaultError>;

    /// Encrypts `plaintext` with the store's key, with `aad` as additional data.
    fn seal(&self, policy: &Policy, aad: &[u8], plaintext: &[u8]) -> Result<Sealed, VaultError>;

    /// Decrypts `sealed` with the store's key. A failed tag is [`VaultError::Tag`].
    fn open(
        &self,
        policy: &Policy,
        aad: &[u8],
        sealed: &Sealed,
    ) -> Result<Zeroizing<Vec<u8>>, VaultError>;
}

/// What a store does with its key, from [`AndroidOptions`](crate::AndroidOptions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Policy {
    namespace: String,
    /// Clears the store when its key is permanently invalid. Only an explicit opt-in.
    pub(crate) reset_on_error: bool,
    /// Whether using the key needs a biometric or device-credential check.
    pub(crate) user_auth: bool,
    /// Whether every use checks again, with no unlock kept between calls.
    pub(crate) per_operation: bool,
    /// Whether only strong biometrics count, and device credentials are refused.
    pub(crate) strong_only: bool,
    /// Whether the key must live in StrongBox.
    pub(crate) strongbox: bool,
    /// Whether a passive biometric match needs one more tap (Android 10 and up).
    pub(crate) confirmation_required: bool,
    pub(crate) prompt_title: String,
    pub(crate) prompt_subtitle: String,
    pub(crate) prompt_negative: String,
}

impl Policy {
    /// The policy of the platform's options. Empty or missing names take their defaults.
    pub(crate) fn from_options(options: &PlatformOptions) -> Self {
        let android = &options.android;
        let or_default = |value: &Option<String>, default: &str| {
            value
                .clone()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| default.to_owned())
        };
        Self {
            namespace: or_default(&android.storage_namespace, DEFAULT_NAMESPACE),
            reset_on_error: android.reset_on_error,
            user_auth: android.enforce_biometrics || android.require_biometrics_per_operation,
            per_operation: android.require_biometrics_per_operation,
            strong_only: android.biometric_type == AndroidBiometricType::StrongBiometricOnly,
            strongbox: android.require_strong_box,
            confirmation_required: android.require_biometric_confirmation,
            prompt_title: or_default(&android.biometric_prompt_title, "Authenticate to access"),
            prompt_subtitle: or_default(
                &android.biometric_prompt_subtitle,
                "Use biometrics or device credentials",
            ),
            prompt_negative: or_default(&android.biometric_prompt_negative_button, "Cancel"),
        }
    }

    /// The namespace the key and the data live under.
    pub(crate) fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The AndroidKeyStore alias of the store's key.
    pub(crate) fn alias(&self) -> String {
        format!("nsh-secure-storage.{}", self.namespace)
    }

    /// The name of the store for locking: two handles on one namespace share it.
    pub(crate) fn store_id(&self) -> String {
        format!("android|{}", self.namespace)
    }
}
