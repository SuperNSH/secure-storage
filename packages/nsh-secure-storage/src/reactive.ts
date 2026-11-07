import { batch, cleanup, signal } from "nsh";
import { asStorageError, type SecureStorageError } from "./errors";
import type { SecureStorage } from "./store";

/** Where a `useSecureValue` read stands. */
export type SecureValueState = "loading" | "ready" | "error";

/** A key's value as a reactive read. Reading `value()` in a binding follows this key only. */
export interface SecureValue {
  /** The value, `null` when there is none, and `undefined` until the first read ends. */
  value(): string | null | undefined;
  state(): SecureValueState;
  error(): SecureStorageError | null;
  /** Reads the key again. */
  refresh(): void;
}

export interface UseSecureValueOptions {
  /** The store the key lives in. */
  storage: SecureStorage;
}

/**
 * Reads `key` as a reactive value, and follows the writes made through `storage`. The
 * subscription ends when the component goes (through `cleanup`). An answer from a read that was
 * overtaken by a write or a refresh is dropped, so a slow read never overwrites a newer write.
 */
export function useSecureValue(key: string, options: UseSecureValueOptions): SecureValue {
  const { storage } = options;
  const value = signal<string | null | undefined>(undefined);
  const state = signal<SecureValueState>("loading");
  const failure = signal<SecureStorageError | null>(null);
  let alive = true;
  let latest = 0;

  const show = (next: string | null) => {
    batch(() => {
      value.set(next);
      state.set("ready");
      failure.set(null);
    });
  };

  const stop = storage.registerListener({
    key,
    listener: (next) => {
      latest += 1;
      show(next);
    },
  });

  const load = () => {
    const id = ++latest;
    storage.read({ key }).then(
      (found) => {
        if (alive && id === latest) show(found);
      },
      (error: unknown) => {
        if (!alive || id !== latest) return;
        batch(() => {
          failure.set(asStorageError(error, key));
          state.set("error");
        });
      },
    );
  };

  cleanup(() => {
    alive = false;
    latest += 1;
    stop();
  });

  load();

  return {
    value: () => value(),
    state: () => state(),
    error: () => failure(),
    refresh: () => {
      if (alive) load();
    },
  };
}
