import { NativeSecureStorage, type StoreOptions } from "nsh-secure-storage/native";
import type { BackendFactory, StorageBackend, StoredEntry } from "./backend";
import { asStorageError, storageError } from "./errors";
import type { SecureStorageOptions } from "./options";

/**
 * The Rust crate's store (`nsh-secure-storage/native`), for an app that lists the crate under
 * `"rust"` in `nsh.json`. Importing this module without the crate listed fails the bundle, so
 * import it only from such an app. The Apple options apply on iOS and macOS.
 */
export const rustBackend: BackendFactory = (options) => new RustBackend(options);

function storeOptions(options: SecureStorageOptions): StoreOptions {
  const apple = { ...options.iOptions, ...options.mOptions };
  const result: StoreOptions = {};
  if (apple.accountName !== undefined) result.accountName = apple.accountName;
  if (apple.groupId !== undefined) result.groupId = apple.groupId;
  if (apple.accessibility !== undefined) result.accessibility = apple.accessibility;
  if (apple.synchronizable !== undefined) result.synchronizable = apple.synchronizable;
  if (apple.accessControlFlags !== undefined) result.accessControlFlags = apple.accessControlFlags;
  if (apple.authenticationUI !== undefined) result.authenticationUi = apple.authenticationUI;
  if (apple.operationPrompt !== undefined) result.operationPrompt = apple.operationPrompt;
  if (options.mOptions?.usesDataProtectionKeychain !== undefined) {
    result.usesDataProtectionKeychain = options.mOptions.usesDataProtectionKeychain;
  }
  if (options.mOptions?.keychainPath !== undefined) result.keychainPath = options.mOptions.keychainPath;
  return result;
}

class RustBackend implements StorageBackend {
  private readonly native: NativeSecureStorage;

  constructor(options: SecureStorageOptions) {
    try {
      this.native = new NativeSecureStorage(storeOptions(options));
    } catch {
      throw storageError("ERR_SECURE_STORAGE_NOT_AVAILABLE");
    }
  }

  async read(key: string): Promise<string | null> {
    return this.settle(() => this.native.read(key), key);
  }

  async write(key: string, value: string): Promise<void> {
    await this.settle(() => this.native.write(key, value), key);
  }

  async delete(key: string): Promise<void> {
    await this.settle(() => this.native.delete(key), key);
  }

  async containsKey(key: string): Promise<boolean> {
    return this.settle(() => this.native.containsKey(key), key);
  }

  async readAll(): Promise<StoredEntry[]> {
    return this.settle(() => this.native.readAll());
  }

  async deleteAll(): Promise<void> {
    await this.settle(() => this.native.deleteAll());
  }

  async isProtectedDataAvailable(): Promise<boolean> {
    return this.native.isProtectedDataAvailable();
  }

  onProtectedDataChange(): () => void {
    // The crate exposes the current state, not a change event.
    throw storageError("ERR_SECURE_STORAGE_NOT_AVAILABLE");
  }

  private async settle<T>(operation: () => Promise<T>, key?: string): Promise<T> {
    try {
      return await operation();
    } catch (error) {
      throw asStorageError(error, key);
    }
  }
}
