// The Android half of `nsh-secure-storage/host`: the `secureStorage.*` handlers of an embedding
// app. Values are AES-GCM ciphertext under an AndroidKeyStore key, kept in a file in the app's
// no_backup folder, so Auto Backup and device transfer leave them out. A failed read never
// removes a value; only a failed tag or an invalidated key may, and only with resetOnError.

package dev.nsh.securestorage

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import java.io.File
import java.security.KeyStore
import java.util.Base64
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import org.json.JSONArray
import org.json.JSONObject

/** A failure the handlers reject with: [code] is an `ERR_SECURE_STORAGE_*` code, [detail] says what failed. */
class SecureStorageHostException(val code: String, val detail: String) : Exception("$code: $detail")

/**
 * The handlers of `nsh-secure-storage/host`. Register each name on the embedding app's view:
 *
 *     for (name in SecureStorageHost.NAMES) {
 *         nsh.handle(name) { data, reply -> reply.resolve(host.handle(name, data)) }
 *     }
 *
 * Call [handle] on the thread the handler runs on, and answer with its result or reject with
 * the message of the exception.
 */
class SecureStorageHost(context: Context) {
    private val directory = File(context.noBackupFilesDir, "nsh-secure-storage").apply { mkdirs() }
    private val keyAlias = "nsh-secure-storage-host"

    /** Runs the handler [name] with the call's [data], and returns its JSON answer. */
    fun handle(name: String, data: Any?): Any? {
        val call = data as? Map<*, *> ?: emptyMap<String, Any>()
        val options = (call["options"] as? Map<*, *>)?.get("android") as? Map<*, *>
        val resetOnError = options?.get("resetOnError") == true
        return when (name) {
            "secureStorage.read" -> mapOf("value" to read(keyOf(call), resetOnError))
            "secureStorage.write" -> { write(keyOf(call), valueOf(call)); null }
            "secureStorage.delete" -> { delete(keyOf(call)); null }
            "secureStorage.containsKey" -> mapOf("present" to (read(keyOf(call), resetOnError) != null))
            "secureStorage.readAll" -> mapOf("entries" to load().keys().asSequence().toList().sorted().mapNotNull { key ->
                read(key, resetOnError)?.let { mapOf("key" to key, "value" to it) }
            })
            "secureStorage.deleteAll" -> { save(JSONObject()); null }
            "secureStorage.protectedDataAvailable" -> mapOf("available" to true)
            else -> throw SecureStorageHostException("ERR_SECURE_STORAGE_NOT_AVAILABLE", "unknown handler")
        }
    }

    private fun keyOf(call: Map<*, *>): String {
        val key = call["key"] as? String
        if (key.isNullOrEmpty() || key.contains('\u0000')) {
            throw SecureStorageHostException("ERR_SECURE_STORAGE_INVALID_KEY", "the key is empty or has a NUL")
        }
        return key
    }

    private fun valueOf(call: Map<*, *>): String =
        call["value"] as? String ?: throw SecureStorageHostException("ERR_SECURE_STORAGE_BACKEND", "the value is not a string")

    private fun read(key: String, resetOnError: Boolean): String? {
        val entry = load().optJSONObject(key) ?: return null
        val cipher = Cipher.getInstance(TRANSFORM)
        val iv = Base64.getDecoder().decode(entry.getString("iv"))
        val ciphertext = Base64.getDecoder().decode(entry.getString("ciphertext"))
        return try {
            cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(TAG_BITS, iv))
            String(cipher.doFinal(ciphertext), Charsets.UTF_8)
        } catch (error: AEADBadTagException) {
            if (resetOnError) { delete(key); null }
            else throw SecureStorageHostException("ERR_SECURE_STORAGE_CORRUPT", "the item failed its integrity check")
        } catch (error: KeyPermanentlyInvalidatedException) {
            if (resetOnError) { delete(key); null }
            else throw SecureStorageHostException("ERR_SECURE_STORAGE_KEY_INVALIDATED", "the key was invalidated")
        }
    }

    private fun write(key: String, value: String) {
        val cipher = Cipher.getInstance(TRANSFORM)
        cipher.init(Cipher.ENCRYPT_MODE, key())
        val entry = JSONObject()
            .put("iv", Base64.getEncoder().encodeToString(cipher.iv))
            .put("ciphertext", Base64.getEncoder().encodeToString(cipher.doFinal(value.toByteArray(Charsets.UTF_8))))
        val all = load()
        all.put(key, entry)
        save(all)
    }

    private fun delete(key: String) {
        val all = load()
        all.remove(key)
        save(all)
    }

    /** The file holds `{key: {iv, ciphertext}}`. It is replaced whole, through a temporary file. */
    private fun load(): JSONObject {
        val file = File(directory, FILE)
        if (!file.exists()) return JSONObject()
        return JSONObject(file.readText(Charsets.UTF_8))
    }

    private fun save(all: JSONObject) {
        val temporary = File(directory, "$FILE.tmp")
        temporary.writeText(all.toString(), Charsets.UTF_8)
        if (!temporary.renameTo(File(directory, FILE))) {
            throw SecureStorageHostException("ERR_SECURE_STORAGE_BACKEND", "the values file could not be replaced")
        }
    }

    /** The AndroidKeyStore key, made once and never exported. */
    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(keyAlias, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(
            KeyGenParameterSpec.Builder(keyAlias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build(),
        )
        return generator.generateKey()
    }

    companion object {
        /** The handler names the TSX backend calls. */
        val NAMES = listOf(
            "secureStorage.read", "secureStorage.write", "secureStorage.delete",
            "secureStorage.containsKey", "secureStorage.readAll", "secureStorage.deleteAll",
            "secureStorage.protectedDataAvailable",
        )
        private const val TRANSFORM = "AES/GCM/NoPadding"
        private const val TAG_BITS = 128
        private const val FILE = "values.json"
    }
}
