import { expect, open, test } from "nsh/test";

test("the screen reads its state from the host's store", async () => {
  const items = new Map<string, string>([["remembered", "yes"]]);
  const app = await open("../App.tsx", {
    clock: {},
    host: {
      handlers: {
        "secureStorage.read": ({ key }: { key: string }) => ({ value: items.get(key) ?? null }),
        "secureStorage.write": ({ key, value }: { key: string; value: string }) => {
          items.set(key, value);
          return null;
        },
      },
    },
  });
  await app.waitFor("text:remembered ready");
  expect(app.texts()).toContain("embedded");
});
