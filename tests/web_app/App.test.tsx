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

/** The steps' errors while the page is gone: it reloads, then connects again as a new page. */
const RELOADING = ["no host that takes test steps is connected", "the host left"];

function reloading(error: unknown): boolean {
  return RELOADING.some((message) => String(error).includes(message));
}

/**
 * Presses Reload and waits for the new page. The press's own wait for the app to settle can meet
 * the page gone, and `waitFor` fails at once without a page, so this looks again until "idle",
 * which only a fresh load shows, is on screen.
 */
async function reload(app: Awaited<ReturnType<typeof open>>) {
  try {
    await app.press("role:button[Reload]");
  } catch (error) {
    if (!reloading(error)) throw error;
  }
  const deadline = Date.now() + 10_000;
  for (;;) {
    try {
      if (app.shows("text:idle")) return;
    } catch (error) {
      if (!reloading(error)) throw error;
    }
    if (Date.now() > deadline) throw new Error("the page did not load again in 10 s");
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
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

test("a value written by one page is read by the next load of it", async () => {
  // Each `open` gets storage of its own, so the other page is a reload of the same one.
  const app = await fresh();
  await app.press("role:button[Write]");
  await app.waitFor("text:written");
  await reload(app);
  await app.press("role:button[Read]");
  await app.waitFor("text:read [s3cret-test-value]");
});
