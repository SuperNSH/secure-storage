import type { BackendFactory, StorageBackend, StoredEntry } from "./backend";

/** Options of the memory store. */
export interface MemoryOptions {
  /** The entries the store starts with. */
  initialValues?: Record<string, string>;
  /** How long a read takes, on the timer, in ms. Writes are never delayed. Default 0. */
  readDelayMs?: number;
}

/**
 * A store in memory, for tests and for apps that keep nothing on the device. Its values end with
 * the app. It keeps the order of a real store: a read waits for the writes queued before it, and
 * a write made during a slow read lands at once.
 */
export function memoryBackend(options: MemoryOptions = {}): BackendFactory {
  return () => new MemoryBackend(options);
}

class MemoryBackend implements StorageBackend {
  private readonly values: Map<string, string>;
  private readonly readDelayMs: number;
  private queue: Promise<unknown> = Promise.resolve();

  constructor(options: MemoryOptions) {
    this.values = new Map(Object.entries(options.initialValues ?? {}));
    this.readDelayMs = options.readDelayMs ?? 0;
  }

  /** Runs `operation` after the writes queued before it. */
  private inOrder<T>(operation: () => T): Promise<T> {
    const run = this.queue.then(operation);
    this.queue = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
  }

  read(key: string): Promise<string | null> {
    // The answer is taken once the writes before it are done, and the queue is not held while it
    // waits, so a write made during the delay lands at once.
    const snapshot = this.queue.then(() => this.values.get(key) ?? null);
    if (this.readDelayMs === 0) return snapshot;
    return snapshot.then(
      (value) =>
        new Promise<string | null>((resolve) => {
          setTimeout(() => resolve(value), this.readDelayMs);
        }),
    );
  }

  write(key: string, value: string): Promise<void> {
    return this.inOrder(() => {
      this.values.set(key, value);
    });
  }

  delete(key: string): Promise<void> {
    return this.inOrder(() => {
      this.values.delete(key);
    });
  }

  containsKey(key: string): Promise<boolean> {
    return this.inOrder(() => this.values.has(key));
  }

  readAll(): Promise<StoredEntry[]> {
    return this.inOrder(() =>
      [...this.values.entries()]
        .map(([key, value]) => ({ key, value }))
        .sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0)),
    );
  }

  deleteAll(): Promise<void> {
    return this.inOrder(() => {
      this.values.clear();
    });
  }

  isProtectedDataAvailable(): Promise<boolean> {
    return Promise.resolve(true);
  }

  onProtectedDataChange(): () => void {
    // Memory has no protected data, so it never changes.
    return () => {};
  }
}
