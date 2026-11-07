/** The codes a secure-storage rejection carries. Apps match on `code`. */
export type SecureStorageErrorCode =
  | "ERR_SECURE_STORAGE_LOCKED"
  | "ERR_SECURE_STORAGE_NOT_AVAILABLE"
  | "ERR_SECURE_STORAGE_KEY_INVALIDATED"
  | "ERR_SECURE_STORAGE_CORRUPT"
  | "ERR_SECURE_STORAGE_AUTH_CANCELLED"
  | "ERR_SECURE_STORAGE_AUTH_FAILED"
  | "ERR_SECURE_STORAGE_MISSING_ENTITLEMENT"
  | "ERR_SECURE_STORAGE_INVALID_KEY"
  | "ERR_SECURE_STORAGE_BACKEND";

/** Same wording as the Rust crate's messages. None holds a value. */
const MESSAGES: Record<SecureStorageErrorCode, string> = {
  ERR_SECURE_STORAGE_LOCKED: "The store is locked. Unlock the device and try again.",
  ERR_SECURE_STORAGE_NOT_AVAILABLE:
    "This platform cannot do this here. Check the options and the platform's support.",
  ERR_SECURE_STORAGE_KEY_INVALIDATED:
    "The key for this data was invalidated, so the data cannot be read. Delete the key and write it again.",
  ERR_SECURE_STORAGE_CORRUPT: "The stored data is damaged and cannot be decoded.",
  ERR_SECURE_STORAGE_AUTH_CANCELLED: "The user cancelled the authentication prompt.",
  ERR_SECURE_STORAGE_AUTH_FAILED: "Authentication failed.",
  ERR_SECURE_STORAGE_MISSING_ENTITLEMENT:
    "The app is not signed with the keychain access group it asks for. Add that group to the app's keychain-access-groups entitlement and sign the app with it.",
  ERR_SECURE_STORAGE_INVALID_KEY: "The key must not be empty or contain a NUL character.",
  ERR_SECURE_STORAGE_BACKEND: "The platform's secret store failed.",
};

const KNOWN = new Set<string>(Object.keys(MESSAGES));

/** Codes nsh itself raises that pass through unchanged. */
const PASS_THROUGH = new Set<string>([
  "ERR_ACCESS_DENIED",
  "ERR_FEATURE_UNAVAILABLE_ON_PLATFORM",
  "ETIMEDOUT",
]);

/** A rejection of the secure store. `key` names the key, never the value. */
export class SecureStorageError extends Error {
  readonly code: string;
  readonly key: string | undefined;

  constructor(code: string, message: string, key?: string) {
    super(key === undefined ? message : `${message} (key ${JSON.stringify(key)})`);
    this.name = "SecureStorageError";
    this.code = code;
    this.key = key;
  }
}

/** The error for a known code, with its standard message. */
export function storageError(code: SecureStorageErrorCode, key?: string): SecureStorageError {
  return new SecureStorageError(code, MESSAGES[code], key);
}

/**
 * Turns whatever a backend or the host threw into a coded `SecureStorageError`. A host's
 * rejection carries `ERR_NSH_HOST_ERROR`, and its message starts with the code the handler
 * wrote, as `ERR_SECURE_STORAGE_LOCKED: text`.
 */
export function asStorageError(error: unknown, key?: string): SecureStorageError {
  if (error instanceof SecureStorageError) return error;
  const raw = (error ?? {}) as { code?: unknown; message?: unknown };
  const code = typeof raw.code === "string" ? raw.code : "";
  const message = typeof raw.message === "string" ? raw.message : "";
  if (code === "ERR_NSH_HOST_ERROR") {
    const named = /^(ERR_SECURE_STORAGE_[A-Z_]+): /.exec(message)?.[1];
    if (named !== undefined && KNOWN.has(named)) return storageError(named as SecureStorageErrorCode, key);
    return storageError("ERR_SECURE_STORAGE_BACKEND", key);
  }
  if (KNOWN.has(code)) return storageError(code as SecureStorageErrorCode, key);
  if (PASS_THROUGH.has(code)) return new SecureStorageError(code, message, key);
  return storageError("ERR_SECURE_STORAGE_BACKEND", key);
}

/** Throws `ERR_SECURE_STORAGE_INVALID_KEY` for a key the stores cannot hold. */
export function checkKey(key: unknown): asserts key is string {
  if (typeof key !== "string" || key.length === 0 || key.includes("\u0000")) {
    throw storageError(
      "ERR_SECURE_STORAGE_INVALID_KEY",
      typeof key === "string" ? key : undefined,
    );
  }
}
