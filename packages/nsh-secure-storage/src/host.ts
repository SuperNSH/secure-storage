import { host } from "nsh";
import type { BackendFactory, StorageBackend, StoredEntry } from "./backend";
import { asStorageError } from "./errors";
import type { SecureStorageOptions } from "./options";

/**
 * The embedding app's secure store, reached through its `secureStorage.*` host handlers. Use it
 * in a signed TSX package that runs inside a host app, which implements the handlers with the
 * Keychain or AndroidKeyStore (`native/ios/SecureStorageHost.swift`,
 * `native/android/SecureStorageHost.kt`). The app must declare `"host": ["secureStorage.*"]`.
 */
export const hostBackend: BackendFactory = (options) => new HostBackend(options);

/** The handler names. Each call carries the store's platform options under `options`. */
const NAMES = {
  read: "secureStorage.read",
  write: "secureStorage.write",
  delete: "secureStorage.delete",
  containsKey: "secureStorage.containsKey",
  readAll: "secureStorage.readAll",
  deleteAll: "secureStorage.deleteAll",
  protectedDataAvailable: "secureStorage.protectedDataAvailable",
  protectedDataChanged: "secureStorage.protectedDataChanged",
} as const;

class HostBackend implements StorageBackend {
  private readonly options: Record<string, unknown>;

  constructor(options: SecureStorageOptions) {
    this.options = {
      ios: options.iOptions ?? null,
      macos: options.mOptions ?? null,
      android: options.aOptions ?? null,
    };
  }

  private call(name: string, data: Record<string, unknown>, key?: string): Promise<unknown> {
    return host.call(name, { ...data, options: this.options }).catch((error: unknown) => {
      throw asStorageError(error, key);
    });
  }

  async read(key: string): Promise<string | null> {
    const reply = (await this.call(NAMES.read, { key }, key)) as { value: string | null } | null;
    return reply?.value ?? null;
  }

  async write(key: string, value: string): Promise<void> {
    await this.call(NAMES.write, { key, value }, key);
  }

  async delete(key: string): Promise<void> {
    await this.call(NAMES.delete, { key }, key);
  }

  async containsKey(key: string): Promise<boolean> {
    const reply = (await this.call(NAMES.containsKey, { key }, key)) as { present: boolean } | null;
    return reply?.present === true;
  }

  async readAll(): Promise<StoredEntry[]> {
    const reply = (await this.call(NAMES.readAll, {})) as { entries: StoredEntry[] } | null;
    return reply?.entries ?? [];
  }

  async deleteAll(): Promise<void> {
    await this.call(NAMES.deleteAll, {});
  }

  async isProtectedDataAvailable(): Promise<boolean> {
    const reply = (await this.call(NAMES.protectedDataAvailable, {})) as { available: boolean } | null;
    return reply?.available === true;
  }

  onProtectedDataChange(listener: (available: boolean) => void): () => void {
    return host.on(NAMES.protectedDataChanged, (event) => {
      listener((event as { available?: unknown } | null)?.available === true);
    });
  }
}
