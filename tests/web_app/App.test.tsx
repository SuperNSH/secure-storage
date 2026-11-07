// Browser tests of the web backend, run by `nsh test --target web`. Each step is a press on the
// app's buttons, and each check reads what the app shows. Nothing here reads storage directly.
import { expect, open, test } from "nsh/test";

/** Opens the app and clears its store, so each test starts with no values. */
async function fresh() {
  const app = await open("./App.tsx");
  await app.press("role:button[Delete all]");
  await app.waitFor("text:cleared");
  return app;
}

test("a value is written, read, listed, checked and deleted", async () => {
  const app = await fresh();
  await app.press("role:button[Write]");
  await app.waitFor("text:written");
  await app.press("role:button[Read]");
  await app.waitFor("text:read [s3cret-test-value]");
  await app.press("role:button[List]");
  await app.waitFor("text:list [token]");
  await app.press("role:button[Contains]");
  await app.waitFor("text:contains true");
  await app.press("role:button[Delete]");
  await app.waitFor("text:deleted");
  await app.press("role:button[Contains]");
  await app.waitFor("text:contains false");
  await app.press("role:button[Read]");
  await app.waitFor("text:read [none]");
});

test("an empty value is a value, not an absent one", async () => {
  const app = await fresh();
  await app.press("role:button[Write empty]");
  await app.waitFor("text:empty written");
  await app.press("role:button[Read]");
  await app.waitFor("text:read []");
  await app.press("role:button[Contains]");
  await app.waitFor("text:contains true");
});

test("several writes at once all land", async () => {
  const app = await fresh();
  await app.press("role:button[Write three]");
  await app.waitFor("text:three written");
  await app.press("role:button[List]");
  await app.waitFor("text:list [a,b,c]");
});

test("delete all removes every value", async () => {
  const app = await fresh();
  await app.press("role:button[Write three]");
  await app.waitFor("text:three written");
  await app.press("role:button[Delete all]");
  await app.waitFor("text:cleared");
  await app.press("role:button[List]");
  await app.waitFor("text:list []");
});

test("a key with a NUL character is refused with its code", async () => {
  const app = await fresh();
  await app.press("role:button[Write nul]");
  await app.waitFor("text:error ERR_SECURE_STORAGE_INVALID_KEY");
});

test("a value written by one page is read by another", async () => {
  const writer = await fresh();
  await writer.press("role:button[Write]");
  await writer.waitFor("text:written");
  const reader = await open("./App.tsx");
  await reader.press("role:button[Read]");
  await reader.waitFor("text:read [s3cret-test-value]");
});
