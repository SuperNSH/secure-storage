//! WebCrypto: the store's AES-256-GCM key, and sealing and opening of one value. A record is
//! `[version][12-byte IV][ciphertext and 16-byte tag]`. The store and the key name are the AAD, so
//! a record moved under another key name does not open.

use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AesGcmParams, AesKeyGenParams, CryptoKey, SubtleCrypto};
use zeroize::Zeroizing;

use super::{backend_error, db::Db, unavailable};
use crate::error::{ErrorKind, SecureStorageError};

const RECORD_VERSION: u8 = 1;
const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;

/// Refuses a page that is not a secure context. WebCrypto is missing there, so nothing is sealed.
pub(super) fn require_secure_context() -> Result<(), SecureStorageError> {
    let window = web_sys::window().ok_or_else(unavailable)?;
    if !window.is_secure_context() || subtle().is_err() {
        return Err(SecureStorageError::new(ErrorKind::InsecureContext));
    }
    Ok(())
}

/// The page's `crypto.subtle`.
fn subtle() -> Result<SubtleCrypto, SecureStorageError> {
    let crypto = web_sys::window()
        .ok_or_else(unavailable)?
        .crypto()
        .map_err(|_| unavailable())?;
    Reflect::get(&crypto, &JsValue::from_str("subtle"))
        .ok()
        .and_then(|subtle| subtle.dyn_into::<SubtleCrypto>().ok())
        .ok_or_else(unavailable)
}

/// The data a record is bound to: the store's name, then the key's.
pub(super) fn aad(store: &str, key: &str) -> Vec<u8> {
    let mut aad = (store.len() as u64).to_be_bytes().to_vec();
    aad.extend_from_slice(store.as_bytes());
    aad.extend_from_slice(key.as_bytes());
    aad
}

/// The store's key. A first write creates it; a race between first writes keeps the first one
/// stored, and every writer uses that one.
pub(super) async fn load_or_create_key(
    db: &Db,
    store: &str,
) -> Result<CryptoKey, SecureStorageError> {
    if let Some(key) = db.get_key(store).await? {
        return Ok(key);
    }
    let candidate = generate().await?;
    if db.add_key(store, &candidate).await? {
        return Ok(candidate);
    }
    db.get_key(store).await?.ok_or_else(backend_error)
}

/// A new AES-256-GCM key. It is not extractable: the page can use it, never read its bytes.
async fn generate() -> Result<CryptoKey, SecureStorageError> {
    let params = AesKeyGenParams::new("AES-GCM", 256);
    let usages = Array::of2(&JsValue::from_str("encrypt"), &JsValue::from_str("decrypt"));
    let promise = subtle()?
        .generate_key_with_object(&params, false, &usages)
        .map_err(|_| backend_error())?;
    JsFuture::from(promise)
        .await
        .map_err(|_| backend_error())?
        .dyn_into::<CryptoKey>()
        .map_err(|_| backend_error())
}

/// Encrypts `plaintext` with a fresh random IV, and returns the record.
pub(super) async fn seal(
    key: &CryptoKey,
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, SecureStorageError> {
    let mut iv = [0u8; IV_LEN];
    web_sys::window()
        .ok_or_else(unavailable)?
        .crypto()
        .and_then(|crypto| crypto.get_random_values_with_u8_array(&mut iv))
        .map_err(|_| backend_error())?;
    let params = gcm_params(&iv, aad);
    let data = Zeroizing::new(plaintext.to_vec());
    let promise = subtle()?
        .encrypt_with_object_and_u8_array(&params, key, &data[..])
        .map_err(|_| backend_error())?;
    let ciphertext = JsFuture::from(promise).await.map_err(|_| backend_error())?;
    let mut record = Vec::with_capacity(1 + IV_LEN + plaintext.len() + TAG_LEN);
    record.push(RECORD_VERSION);
    record.extend_from_slice(&iv);
    record.extend_from_slice(&Uint8Array::new(&ciphertext).to_vec());
    Ok(record)
}

/// Decrypts a record. `None` when it is malformed, fails its tag, or is not UTF-8: the caller
/// reads that as absent.
pub(super) async fn open(key: &CryptoKey, aad: &[u8], record: &[u8]) -> Option<Zeroizing<String>> {
    let (&version, rest) = record.split_first()?;
    if version != RECORD_VERSION || rest.len() < IV_LEN + TAG_LEN {
        return None;
    }
    let (iv, ciphertext) = rest.split_at(IV_LEN);
    let params = gcm_params(iv, aad);
    let data = ciphertext.to_vec();
    let promise = subtle()
        .ok()?
        .decrypt_with_object_and_u8_array(&params, key, &data[..])
        .ok()?;
    let plaintext = JsFuture::from(promise).await.ok()?;
    let bytes = Zeroizing::new(Uint8Array::new(&plaintext).to_vec());
    let text = std::str::from_utf8(&bytes).ok()?;
    Some(Zeroizing::new(text.to_owned()))
}

fn gcm_params(iv: &[u8], aad: &[u8]) -> AesGcmParams {
    let params = AesGcmParams::new_with_u8_array("AES-GCM", &Uint8Array::from(iv));
    let aad = Uint8Array::from(aad);
    params.set_additional_data(aad.unchecked_ref::<Object>());
    params
}

/// Logs that an entry cannot be read. The log names the key and never holds a value.
pub(super) fn log_undecryptable(key: &str) {
    web_sys::console::warn_1(&JsValue::from_str(&format!(
        "nsh-secure-storage: the entry {key:?} cannot be decrypted, so it reads as absent (ERR_SECURE_STORAGE_CORRUPT)"
    )));
}
