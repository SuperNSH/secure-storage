//! The AndroidKeyStore, over JNI. This file is the only one that touches the JVM.
//!
//! An NSH app has no JVM of its own, so the VM is found once, through `libnativehelper`, which the
//! Android runtime loads for every app. Each call attaches its thread. A Java exception that is
//! left pending becomes a [`VaultError`], by the class of the exception.

use std::{ffi::c_void, path::PathBuf, sync::OnceLock};

use jni::{
    Env, JavaVM, jni_sig, jni_str,
    objects::{JByteArray, JObject, JValue},
    sys::{jint, jsize},
};
use zeroize::Zeroizing;

use super::keystore::{Keystore, Policy, Sealed, VaultError};

/// `JNI_GetCreatedJavaVMs`, found by name in `libnativehelper`.
type GetCreatedVms = unsafe extern "C" fn(*mut *mut jni::sys::JavaVM, jsize, *mut jsize) -> jint;

/// `KeyProperties.PURPOSE_ENCRYPT | PURPOSE_DECRYPT`.
const PURPOSES: jint = 3;
/// `Cipher.ENCRYPT_MODE` and `Cipher.DECRYPT_MODE`.
const ENCRYPT_MODE: jint = 1;
const DECRYPT_MODE: jint = 2;
/// `KeyProperties.AUTH_DEVICE_CREDENTIAL` and `AUTH_BIOMETRIC_STRONG` (API 30).
const AUTH_DEVICE_CREDENTIAL: jint = 1;
const AUTH_BIOMETRIC_STRONG: jint = 2;
/// How long a key stays unlocked after a check, when a check is not per operation. A check per
/// operation uses zero, which needs a CryptoObject on every use.
const UNLOCK_SECONDS: jint = 30;

impl From<jni::errors::Error> for VaultError {
    /// A failure to call at all, with no Java exception to classify, is a plain failure.
    fn from(_: jni::errors::Error) -> Self {
        Self::Failed
    }
}

/// Maps a failed JNI call to the error of the Java exception it left pending, if any.
fn pending(env: &mut Env<'_>) -> VaultError {
    let Some(throwable) = env.exception_occurred() else {
        return VaultError::Failed;
    };
    env.exception_clear();
    let classes = [
        (
            jni_str!("android/security/keystore/KeyPermanentlyInvalidatedException"),
            VaultError::Invalidated,
        ),
        (
            jni_str!("javax/crypto/AEADBadTagException"),
            VaultError::Tag,
        ),
        (
            jni_str!("android/security/keystore/UserNotAuthenticatedException"),
            VaultError::Locked,
        ),
        (
            jni_str!("android/security/keystore/StrongBoxUnavailableException"),
            VaultError::NoStrongBox,
        ),
    ];
    for (name, error) in classes {
        let Ok(class) = env.find_class(name) else {
            continue;
        };
        if env.is_instance_of(&throwable, &class).unwrap_or(false) {
            return error;
        }
    }
    VaultError::Failed
}

/// Returns the value of a JNI call, or the error of the exception it left pending.
macro_rules! jtry {
    ($env:expr, $call:expr) => {
        match $call {
            Ok(value) => value,
            Err(_) => return Err(pending($env)),
        }
    };
}

/// The JavaVM of this process, found once and kept.
fn vm() -> Result<JavaVM, VaultError> {
    static RAW: OnceLock<usize> = OnceLock::new();
    if let Some(raw) = RAW.get() {
        // SAFETY: the pointer came from the runtime's own `JNI_GetCreatedJavaVMs`, and the VM lives as long as the process.
        return Ok(unsafe { JavaVM::from_raw(*raw as *mut jni::sys::JavaVM) });
    }
    let raw = find_vm()?;
    let _ = RAW.set(raw as usize);
    // SAFETY: as above.
    Ok(unsafe { JavaVM::from_raw(raw) })
}

fn find_vm() -> Result<*mut jni::sys::JavaVM, VaultError> {
    // SAFETY: `dlopen` and `dlsym` take valid C strings. The symbol is called with the signature
    // `JNI_GetCreatedJavaVMs` has in the JNI specification.
    unsafe {
        let library = libc::dlopen(c"libnativehelper.so".as_ptr(), libc::RTLD_NOW);
        if library.is_null() {
            return Err(VaultError::Failed);
        }
        let symbol = libc::dlsym(library, c"JNI_GetCreatedJavaVMs".as_ptr());
        if symbol.is_null() {
            return Err(VaultError::Failed);
        }
        let get: GetCreatedVms = std::mem::transmute::<*mut c_void, GetCreatedVms>(symbol);
        let mut vm = std::ptr::null_mut();
        let mut count: jsize = 0;
        if get(&mut vm, 1, &mut count) != 0 || count < 1 || vm.is_null() {
            return Err(VaultError::Failed);
        }
        Ok(vm)
    }
}

/// Runs `body` on this thread, attached to the VM.
fn attached<T>(body: impl FnOnce(&mut Env<'_>) -> Result<T, VaultError>) -> Result<T, VaultError> {
    vm()?.attach_current_thread(|env| body(env))
}

/// The AndroidKeyStore, loaded.
fn keystore<'local>(env: &mut Env<'local>) -> Result<JObject<'local>, VaultError> {
    let provider = string(env, "AndroidKeyStore")?;
    let store = jtry!(
        env,
        env.call_static_method(
            jni_str!("java/security/KeyStore"),
            jni_str!("getInstance"),
            jni_sig!("(Ljava/lang/String;)Ljava/security/KeyStore;"),
            &[JValue::Object(&provider)],
        )
    );
    let store = jtry!(env, store.l());
    let null = JObject::null();
    jtry!(
        env,
        env.call_method(
            &store,
            jni_str!("load"),
            jni_sig!("(Ljava/io/InputStream;[C)V"),
            &[JValue::Object(&null), JValue::Object(&null)],
        )
    );
    Ok(store)
}

/// The store's key, or `None` when the keystore has none.
fn find_key<'local>(
    env: &mut Env<'local>,
    store: &JObject<'_>,
    alias: &JObject<'_>,
) -> Result<Option<JObject<'local>>, VaultError> {
    let null = JObject::null();
    let key = jtry!(
        env,
        env.call_method(
            store,
            jni_str!("getKey"),
            jni_sig!("(Ljava/lang/String;[C)Ljava/security/Key;"),
            &[JValue::Object(alias), JValue::Object(&null)],
        )
    );
    let key = jtry!(env, key.l());
    Ok(if key.is_null() { None } else { Some(key) })
}

/// Makes a Java `String` of `text`.
fn string<'local>(env: &mut Env<'local>, text: &str) -> Result<JObject<'local>, VaultError> {
    Ok(jtry!(env, env.new_string(text)).into())
}

/// A Java `String[]` holding `items`.
fn string_array<'local>(
    env: &mut Env<'local>,
    items: &[&str],
) -> Result<jni::objects::JObjectArray<'local>, VaultError> {
    let length = jsize::try_from(items.len()).map_err(|_| VaultError::Failed)?;
    let array = jtry!(
        env,
        env.new_object_array(length, jni_str!("java/lang/String"), JObject::null())
    );
    for (index, item) in items.iter().enumerate() {
        let text = string(env, item)?;
        jtry!(env, array.set_element(env, index, &text));
    }
    Ok(array)
}

/// The API level of the device (`Build.VERSION.SDK_INT`).
fn api_level(env: &mut Env<'_>) -> Result<jint, VaultError> {
    let level = jtry!(
        env,
        env.get_static_field(
            jni_str!("android/os/Build$VERSION"),
            jni_str!("SDK_INT"),
            jni_sig!("I"),
        )
    );
    Ok(jtry!(env, level.i()))
}

/// Copies a Java `byte[]` out, then clears the Java copy.
fn take_bytes(env: &mut Env<'_>, object: JObject<'_>) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let array = jtry!(env, env.cast_local::<JByteArray>(object));
    let bytes = jtry!(env, env.convert_byte_array(&array));
    jtry!(
        env,
        env.call_static_method(
            jni_str!("java/util/Arrays"),
            jni_str!("fill"),
            jni_sig!("([BB)V"),
            &[JValue::Object(&array), JValue::Byte(0)],
        )
    );
    Ok(Zeroizing::new(bytes))
}

/// The Keystore, through JNI.
pub(crate) struct Jni;

impl Keystore for Jni {
    fn no_backup_dir(&self) -> Result<PathBuf, VaultError> {
        attached(|env| {
            let application = jtry!(
                env,
                env.call_static_method(
                    jni_str!("android/app/ActivityThread"),
                    jni_str!("currentApplication"),
                    jni_sig!("()Landroid/app/Application;"),
                    &[],
                )
            );
            let application = jtry!(env, application.l());
            if application.is_null() {
                return Err(VaultError::Failed);
            }
            let folder = jtry!(
                env,
                env.call_method(
                    &application,
                    jni_str!("getNoBackupFilesDir"),
                    jni_sig!("()Ljava/io/File;"),
                    &[],
                )
            );
            let folder = jtry!(env, folder.l());
            let path = jtry!(
                env,
                env.call_method(
                    &folder,
                    jni_str!("getAbsolutePath"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )
            );
            let path = jtry!(env, path.l());
            let path = jtry!(env, env.cast_local::<jni::objects::JString>(path));
            let text = jtry!(env, path.try_to_string(env));
            Ok(PathBuf::from(text))
        })
    }

    fn has_key(&self, policy: &Policy) -> Result<bool, VaultError> {
        attached(|env| {
            let store = keystore(env)?;
            let alias = string(env, &policy.alias())?;
            Ok(find_key(env, &store, &alias)?.is_some())
        })
    }

    fn create_key(&self, policy: &Policy) -> Result<(), VaultError> {
        attached(|env| {
            let alias = string(env, &policy.alias())?;
            let algorithm = string(env, "AES")?;
            let provider = string(env, "AndroidKeyStore")?;
            let generator = jtry!(
                env,
                env.call_static_method(
                    jni_str!("javax/crypto/KeyGenerator"),
                    jni_str!("getInstance"),
                    jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Ljavax/crypto/KeyGenerator;"),
                    &[JValue::Object(&algorithm), JValue::Object(&provider)],
                )
            );
            let generator = jtry!(env, generator.l());

            let builder = jtry!(
                env,
                env.new_object(
                    jni_str!("android/security/keystore/KeyGenParameterSpec$Builder"),
                    jni_sig!("(Ljava/lang/String;I)V"),
                    &[JValue::Object(&alias), JValue::Int(PURPOSES)],
                )
            );
            let modes = string_array(env, &["GCM"])?;
            let paddings = string_array(env, &["NoPadding"])?;
            jtry!(
                env,
                env.call_method(
                    &builder,
                    jni_str!("setBlockModes"),
                    jni_sig!("([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
                    &[JValue::Object(&modes)],
                )
            );
            jtry!(
                env,
                env.call_method(
                    &builder,
                    jni_str!("setEncryptionPaddings"),
                    jni_sig!("([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
                    &[JValue::Object(&paddings)],
                )
            );
            jtry!(
                env,
                env.call_method(
                    &builder,
                    jni_str!("setKeySize"),
                    jni_sig!("(I)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
                    &[JValue::Int(256)],
                )
            );
            jtry!(
                env,
                env.call_method(
                    &builder,
                    jni_str!("setInvalidatedByBiometricEnrollment"),
                    jni_sig!("(Z)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
                    &[JValue::Bool(policy.user_auth)],
                )
            );
            if policy.user_auth {
                jtry!(
                    env,
                    env.call_method(
                        &builder,
                        jni_str!("setUserAuthenticationRequired"),
                        jni_sig!("(Z)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
                        &[JValue::Bool(true)],
                    )
                );
                let level = api_level(env)?;
                let types = if policy.strong_only {
                    AUTH_BIOMETRIC_STRONG
                } else {
                    AUTH_BIOMETRIC_STRONG | AUTH_DEVICE_CREDENTIAL
                };
                if level >= 30 {
                    let timeout = if policy.per_operation {
                        0
                    } else {
                        UNLOCK_SECONDS
                    };
                    jtry!(
                        env,
                        env.call_method(
                            &builder,
                            jni_str!("setUserAuthenticationParameters"),
                            jni_sig!("(II)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
                            &[JValue::Int(timeout), JValue::Int(types)],
                        )
                    );
                } else {
                    let seconds = if policy.per_operation {
                        -1
                    } else {
                        UNLOCK_SECONDS
                    };
                    jtry!(
                        env,
                        env.call_method(
                            &builder,
                            jni_str!("setUserAuthenticationValidityDurationSeconds"),
                            jni_sig!("(I)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
                            &[JValue::Int(seconds)],
                        )
                    );
                }
            }
            if policy.strongbox {
                jtry!(
                    env,
                    env.call_method(
                        &builder,
                        jni_str!("setIsStrongBoxBacked"),
                        jni_sig!("(Z)Landroid/security/keystore/KeyGenParameterSpec$Builder;"),
                        &[JValue::Bool(true)],
                    )
                );
            }
            let spec = jtry!(
                env,
                env.call_method(
                    &builder,
                    jni_str!("build"),
                    jni_sig!("()Landroid/security/keystore/KeyGenParameterSpec;"),
                    &[],
                )
            );
            let spec = jtry!(env, spec.l());
            jtry!(
                env,
                env.call_method(
                    &generator,
                    jni_str!("init"),
                    jni_sig!("(Ljava/security/spec/AlgorithmParameterSpec;)V"),
                    &[JValue::Object(&spec)],
                )
            );
            jtry!(
                env,
                env.call_method(
                    &generator,
                    jni_str!("generateKey"),
                    jni_sig!("()Ljavax/crypto/SecretKey;"),
                    &[],
                )
            );
            Ok(())
        })
    }

    fn delete_key(&self, policy: &Policy) -> Result<(), VaultError> {
        attached(|env| {
            let store = keystore(env)?;
            let alias = string(env, &policy.alias())?;
            jtry!(
                env,
                env.call_method(
                    &store,
                    jni_str!("deleteEntry"),
                    jni_sig!("(Ljava/lang/String;)V"),
                    &[JValue::Object(&alias)],
                )
            );
            Ok(())
        })
    }

    fn seal(&self, policy: &Policy, aad: &[u8], plaintext: &[u8]) -> Result<Sealed, VaultError> {
        attached(|env| {
            let store = keystore(env)?;
            let alias = string(env, &policy.alias())?;
            let key = find_key(env, &store, &alias)?.ok_or(VaultError::Failed)?;
            let transformation = string(env, "AES/GCM/NoPadding")?;
            let cipher = jtry!(
                env,
                env.call_static_method(
                    jni_str!("javax/crypto/Cipher"),
                    jni_str!("getInstance"),
                    jni_sig!("(Ljava/lang/String;)Ljavax/crypto/Cipher;"),
                    &[JValue::Object(&transformation)],
                )
            );
            let cipher = jtry!(env, cipher.l());
            jtry!(
                env,
                env.call_method(
                    &cipher,
                    jni_str!("init"),
                    jni_sig!("(ILjava/security/Key;)V"),
                    &[JValue::Int(ENCRYPT_MODE), JValue::Object(&key)],
                )
            );
            let aad_array = jtry!(env, env.byte_array_from_slice(aad));
            jtry!(
                env,
                env.call_method(
                    &cipher,
                    jni_str!("updateAAD"),
                    jni_sig!("([B)V"),
                    &[JValue::Object(&aad_array)],
                )
            );
            let iv = jtry!(
                env,
                env.call_method(&cipher, jni_str!("getIV"), jni_sig!("()[B"), &[])
            );
            let iv = jtry!(env, iv.l());
            let iv = take_bytes(env, iv)?;
            let input = jtry!(env, env.byte_array_from_slice(plaintext));
            let output = jtry!(
                env,
                env.call_method(
                    &cipher,
                    jni_str!("doFinal"),
                    jni_sig!("([B)[B"),
                    &[JValue::Object(&input)],
                )
            );
            let output = jtry!(env, output.l());
            let data = take_bytes(env, output)?;
            Ok(Sealed {
                iv: iv.to_vec(),
                data: data.to_vec(),
            })
        })
    }

    fn open(
        &self,
        policy: &Policy,
        aad: &[u8],
        sealed: &Sealed,
    ) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        attached(|env| {
            let store = keystore(env)?;
            let alias = string(env, &policy.alias())?;
            let key = find_key(env, &store, &alias)?.ok_or(VaultError::Failed)?;
            let transformation = string(env, "AES/GCM/NoPadding")?;
            let cipher = jtry!(
                env,
                env.call_static_method(
                    jni_str!("javax/crypto/Cipher"),
                    jni_str!("getInstance"),
                    jni_sig!("(Ljava/lang/String;)Ljavax/crypto/Cipher;"),
                    &[JValue::Object(&transformation)],
                )
            );
            let cipher = jtry!(env, cipher.l());
            let iv = jtry!(env, env.byte_array_from_slice(&sealed.iv));
            let spec = jtry!(
                env,
                env.new_object(
                    jni_str!("javax/crypto/spec/GCMParameterSpec"),
                    jni_sig!("(I[B)V"),
                    &[JValue::Int(128), JValue::Object(&iv)],
                )
            );
            jtry!(
                env,
                env.call_method(
                    &cipher,
                    jni_str!("init"),
                    jni_sig!("(ILjava/security/Key;Ljava/security/spec/AlgorithmParameterSpec;)V"),
                    &[
                        JValue::Int(DECRYPT_MODE),
                        JValue::Object(&key),
                        JValue::Object(&spec)
                    ],
                )
            );
            let aad_array = jtry!(env, env.byte_array_from_slice(aad));
            jtry!(
                env,
                env.call_method(
                    &cipher,
                    jni_str!("updateAAD"),
                    jni_sig!("([B)V"),
                    &[JValue::Object(&aad_array)],
                )
            );
            let data = jtry!(env, env.byte_array_from_slice(&sealed.data));
            let output = jtry!(
                env,
                env.call_method(
                    &cipher,
                    jni_str!("doFinal"),
                    jni_sig!("([B)[B"),
                    &[JValue::Object(&data)],
                )
            );
            let output = jtry!(env, output.l());
            take_bytes(env, output)
        })
    }
}
