import { expect, open, test } from "nsh/test";
import { SecureStorage } from "../src";

const line = (texts: string[], prefix: string) => texts.find((t) => t.startsWith(prefix)) ?? "";

test("every method works through the screen", async () => {
  const app = await open("../App.tsx", { clock: {} });
  await app.press("id:write-alpha");
  await app.waitFor("text:wrote alpha");
  await app.press("id:contains-alpha");
  await app.waitFor("text:contains alpha true");
  await app.press("id:read-alpha");
  await app.waitFor("text:read alpha present");
  await app.press("id:read-all");
  await app.waitFor("text:entries 1");
  await app.press("id:write-beta");
  await app.waitFor("text:wrote beta");
  await app.press("id:read-all");
  await app.waitFor("text:entries 2");
  await app.press("id:delete-alpha");
  await app.waitFor("text:deleted alpha");
  await app.press("id:contains-alpha");
  await app.waitFor("text:contains alpha false");
  await app.press("id:delete-all");
  await app.waitFor("text:deleted all");
  await app.press("id:read-all");
  await app.waitFor("text:entries 0");
  await app.press("id:protected");
  await app.waitFor("text:protected true");
});

test("an empty key rejects with its code", async () => {
  const app = await open("../App.tsx", { clock: {} });
  await app.press("id:bad-key");
  await app.waitFor("text:ERR_SECURE_STORAGE_INVALID_KEY");
});

test("a registered listener hears writes and deletes of its key", async () => {
  const app = await open("../App.tsx", { clock: {} });
  await app.press("id:listen");
  await app.press("id:write-beta");
  await app.waitFor("text:heard beta written");
  await app.press("id:delete-all");
  await app.waitFor("text:heard beta removed");
});

test("the store API rejects with a code and never a value", async () => {
  const store = SecureStorage.memory({ secret: "hunter2" });
  await expect(store.read({ key: "" })).rejects.toMatchObject({ code: "ERR_SECURE_STORAGE_INVALID_KEY" });
  await expect(store.write({ key: "a\u0000b", value: "x" })).rejects.toMatchObject({
    code: "ERR_SECURE_STORAGE_INVALID_KEY",
  });
  await store.delete({ key: "missing" });
  const failure = (await store.read({ key: "" }).catch((error: Error) => error)) as Error;
  expect(failure.message.includes("hunter2")).toBe(false);
  expect(await store.read({ key: "secret" })).toBe("hunter2");
});

test("a value written in one store is read back, and a delete leaves no value", async () => {
  const store = SecureStorage.memory();
  await store.write({ key: "k", value: "" });
  expect(await store.read({ key: "k" })).toBe("");
  expect(await store.containsKey({ key: "k" })).toBe(true);
  await store.write({ key: "k", value: null });
  expect(await store.read({ key: "k" })).toBe(null);
  expect(await store.containsKey({ key: "k" })).toBe(false);
});

test("readAll returns the entries in key order", async () => {
  const store = SecureStorage.memory({ b: "2", a: "1", c: "3" });
  expect(await store.readAll()).toEqual([
    { key: "a", value: "1" },
    { key: "b", value: "2" },
    { key: "c", value: "3" },
  ]);
});

test("the screen's texts show the stored state", async () => {
  const app = await open("../App.tsx", { clock: {} });
  await app.waitFor("text:/^beta none/");
  expect(line(app.texts(), "beta").startsWith("beta none")).toBe(true);
});
