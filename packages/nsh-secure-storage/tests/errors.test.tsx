import { expect, test } from "nsh/test";
import { asStorageError, checkKey, storageError } from "../src/errors";

test("a host's coded message keeps its code and drops its text", () => {
  const mapped = asStorageError({
    code: "ERR_NSH_HOST_ERROR",
    message: "ERR_SECURE_STORAGE_LOCKED: secret=hunter2",
  });
  expect(mapped.code).toBe("ERR_SECURE_STORAGE_LOCKED");
  expect(mapped.message.includes("hunter2")).toBe(false);
});

test("an unnamed host failure is a backend error", () => {
  expect(asStorageError({ code: "ERR_NSH_HOST_ERROR", message: "boom" }).code).toBe(
    "ERR_SECURE_STORAGE_BACKEND",
  );
});

test("a refused capability passes its code through", () => {
  const refused = asStorageError({ code: "ERR_ACCESS_DENIED", message: "host refused" });
  expect(refused.code).toBe("ERR_ACCESS_DENIED");
  expect(refused.name).toBe("SecureStorageError");
});

test("an unknown failure becomes a backend error without its text", () => {
  const mapped = asStorageError(new TypeError("value=abc"));
  expect(mapped.code).toBe("ERR_SECURE_STORAGE_BACKEND");
  expect(mapped.message.includes("abc")).toBe(false);
});

test("the key is named, and only the key", () => {
  const error = storageError("ERR_SECURE_STORAGE_LOCKED", "account");
  expect(error.key).toBe("account");
  expect(error.message).toContain('"account"');
});

test("a key must not be empty or contain NUL", () => {
  expect(() => checkKey("")).toThrow();
  expect(() => checkKey("a\u0000")).toThrow();
  expect(() => checkKey("ok key/with:chars")).not.toThrow();
});
