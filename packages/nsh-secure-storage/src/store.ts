import type { BackendFactory, StorageBackend, StoredEntry } from "./backend";
import { asStorageError, checkKey } from "./errors";
import { memoryBackend } from "./memory";
import type { SecureStorageOptions } from "./options";

/** What a store is made from: its options and the backend that keeps the values. */
export interface SecureStorageConfig extends SecureStorageOptions {
  backend: BackendFactory;
}

/** Called with the new value, or `null` after a delete, for one key. */
export type ValueListener = (value: string | null) => void;

/**
 * A handle to one store. Writes and deletes made through it reach its listeners and its
 * `useSecureValue` bindings. Every method rejects with a `SecureStorageError` that has a `code`.
 */
export class SecureStorage {
  private readonly backend: StorageBackend;
  private readonly listeners = new Map<string, Set<ValueListener>>();

  constructor(config: SecureStorageConfig) {
    const { backend, ...options } = config;
    this.backend = backend(options);
  }

  /** A store in memory. Its values end with the app; for tests and for data that stays in memory. */
  static memory(initialValues?: Record<string, string>): SecureStorage {
    return new SecureStorage({ backend: memoryBackend({ initialValues }) });
  }

  /** Stores `value` under `key`. A `null` value deletes the key. */
  async write({ key, value }: { key: string; value: string | null }): Promise<void> {
    checkKey(key);
    try {
      if (value === null) await this.backend.delete(key);
      else await this.backend.write(key, value);
    } catch (error) {
      throw asStorageError(error, key);
    }
    this.notify(key, value);
  }

  /** The value under `key`, or `null` when there is none. */
  async read({ key }: { key: string }): Promise<string | null> {
    checkKey(key);
    try {
      return await this.backend.read(key);
    } catch (error) {
      throw asStorageError(error, key);
    }
  }

  /** Whether a value is stored under `key`. */
  async containsKey({ key }: { key: string }): Promise<boolean> {
    checkKey(key);
    try {
      return await this.backend.containsKey(key);
    } catch (error) {
      throw asStorageError(error, key);
    }
  }

  /** Removes the value under `key`. A key that is not there is not an error. */
  async delete({ key }: { key: string }): Promise<void> {
    checkKey(key);
    try {
      await this.backend.delete(key);
    } catch (error) {
      throw asStorageError(error, key);
    }
    this.notify(key, null);
  }

  /** Every key and value of the store, in key order. */
  async readAll(): Promise<StoredEntry[]> {
    try {
      return await this.backend.readAll();
    } catch (error) {
      throw asStorageError(error);
    }
  }

  /** Removes every value. Listeners of the removed keys hear `null`. */
  async deleteAll(): Promise<void> {
    try {
      // Keys are read only when someone listens: the values stay out of JavaScript otherwise.
      const keys = this.listeners.size === 0 ? [] : (await this.backend.readAll()).map((e) => e.key);
      await this.backend.deleteAll();
      for (const key of keys) this.notify(key, null);
    } catch (error) {
      throw asStorageError(error);
    }
  }

  /**
   * Calls `listener` after each write or delete of `key` made through this handle. Returns the
   * function that ends it. Registering one function twice counts once.
   */
  registerListener({ key, listener }: { key: string; listener: ValueListener }): () => void {
    checkKey(key);
    let set = this.listeners.get(key);
    if (set === undefined) {
      set = new Set();
      this.listeners.set(key, set);
    }
    set.add(listener);
    return () => this.unregisterListener({ key, listener });
  }

  /** Ends one listener of `key`. */
  unregisterListener({ key, listener }: { key: string; listener: ValueListener }): void {
    const set = this.listeners.get(key);
    if (set === undefined) return;
    set.delete(listener);
    if (set.size === 0) this.listeners.delete(key);
  }

  /** Ends every listener of `key`. */
  unregisterAllListenersForKey({ key }: { key: string }): void {
    this.listeners.delete(key);
  }

  /** Ends every listener of this handle. */
  unregisterAllListeners(): void {
    this.listeners.clear();
  }

  /** How many listeners are registered: for one key, or all of them. */
  listenerCount(key?: string): number {
    if (key !== undefined) return this.listeners.get(key)?.size ?? 0;
    let count = 0;
    for (const set of this.listeners.values()) count += set.size;
    return count;
  }

  /** Whether the platform's protected data is readable now. Always `true` where there is none. */
  async isCupertinoProtectedDataAvailable(): Promise<boolean> {
    try {
      return await this.backend.isProtectedDataAvailable();
    } catch (error) {
      throw asStorageError(error);
    }
  }

  /** Calls `listener` when protected data becomes available or unavailable. Returns the stop. */
  onCupertinoProtectedDataAvailabilityChanged(listener: (available: boolean) => void): () => void {
    return this.backend.onProtectedDataChange(listener);
  }

  private notify(key: string, value: string | null): void {
    const set = this.listeners.get(key);
    if (set === undefined) return;
    for (const listener of [...set]) {
      try {
        listener(value);
      } catch {
        // Names the key only: a listener's error text could hold anything.
        console.error(`a secure storage listener for ${JSON.stringify(key)} threw`);
      }
    }
  }
}
