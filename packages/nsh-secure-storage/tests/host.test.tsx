import { expect, open, test } from "nsh/test";

/** A keychain stand-in in the test process: the handlers a host app implements. */
function fakeHost(failRead?: string) {
  const items = new Map<string, string>();
  const handlers = {
    "secureStorage.read": ({ key }: { key: string }) => {
      if (failRead !== undefined) throw new Error(failRead);
      return { value: items.get(key) ?? null };
    },
    "secureStorage.write": ({ key, value }: { key: string; value: string }) => {
      items.set(key, value);
      return null;
    },
    "secureStorage.delete": ({ key }: { key: string }) => {
      items.delete(key);
      return null;
    },
    "secureStorage.containsKey": ({ key }: { key: string }) => ({ present: items.has(key) }),
    "secureStorage.readAll": () => ({
      entries: [...items].map(([key, value]) => ({ key, value })),
    }),
    "secureStorage.deleteAll": () => {
      items.clear();
      return null;
    },
    "secureStorage.protectedDataAvailable": () => ({ available: true }),
  };
  return { items, handlers };
}

test("the screen stores its values through the host's handlers", async () => {
  const keychain = fakeHost();
  const app = await open("../App.tsx", { clock: {}, host: { handlers: keychain.handlers } });
  await app.press("id:write-alpha");
  await app.waitFor("text:wrote alpha");
  expect(keychain.items.get("alpha")).toBe("one");
  await app.press("id:read-alpha");
  await app.waitFor("text:read alpha present");
  const write = app.host.calls().find((call) => call.name === "secureStorage.write");
  expect(write).toMatchObject({ data: { key: "alpha", value: "one" } });
  expect((write!.data as { options?: unknown }).options).toBeDefined();
});

test("a host's coded failure reaches the app with its code", async () => {
  const app = await open("../App.tsx", {
    clock: {},
    host: { handlers: fakeHost("ERR_SECURE_STORAGE_LOCKED: the device is locked").handlers },
  });
  await app.press("id:read-alpha");
  await app.waitFor("text:ERR_SECURE_STORAGE_LOCKED");
});

test("a host with no handler for the store reports a backend error", async () => {
  const app = await open("../App.tsx", { clock: {}, host: { handlers: { analytics: null } } });
  await app.press("id:read-alpha");
  await app.waitFor("text:ERR_SECURE_STORAGE_BACKEND");
});
