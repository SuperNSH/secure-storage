import { open, test } from "nsh/test";

/**
 * A throwaway file keychain, made before the run and never the login keychain:
 *   security create-keychain -p "" .nsh/throwaway.keychain-db
 * The path is relative, so run the test from this directory.
 */
const KEYCHAIN = "/Users/hmziq/fo/nsh-secure-storage/examples/demo-rs/.nsh/throwaway.keychain-db";

test("the Rust store saves, loads and removes a value in a file keychain", async () => {
  const app = await open("../App.tsx", { clock: {}, host: { params: { keychainPath: KEYCHAIN } } });
  await app.press("id:remove");
  await app.waitFor("text:removed");
  await app.press("id:load");
  await app.waitFor("text:absent");
  await app.press("id:save");
  await app.waitFor("text:saved");
  await app.press("id:load");
  await app.waitFor("text:present");
  await app.press("id:remove");
  await app.waitFor("text:removed");
  await app.press("id:load");
  await app.waitFor("text:absent");
});
