import type { SecureStorageOptions } from "./options";

/** One key and its value, as `readAll` returns them. */
export interface StoredEntry {
  key: string;
  value: string;
}

/**
 * What a platform store does. A backend gets validated keys and never a `null` write: `delete`
 * is the removal. Each method rejects with a `SecureStorageError`, or any error `asStorageError`
 * can map.
 */
export interface StorageBackend {
  read(key: string): Promise<string | null>;
  write(key: string, value: string): Promise<void>;
  delete(key: string): Promise<void>;
  containsKey(key: string): Promise<boolean>;
  readAll(): Promise<StoredEntry[]>;
  deleteAll(): Promise<void>;
  isProtectedDataAvailable(): Promise<boolean>;
  /** Throws a coded error synchronously when the backend cannot report changes. */
  onProtectedDataChange(listener: (available: boolean) => void): () => void;
}

/** Makes a backend for a store's options. `new SecureStorage({ backend, ...options })` calls it once. */
export type BackendFactory = (options: SecureStorageOptions) => StorageBackend;
