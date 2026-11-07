import { expect, open, test } from "nsh/test";

const line = (texts: string[], prefix: string) => texts.find((t) => t.startsWith(prefix)) ?? "";

test("a binding on one key does not wake for another key", async () => {
  const app = await open("../App.tsx", { clock: {} });
  await app.waitFor("text:/^beta none/");
  const alphaBefore = line(app.texts(), "alpha");
  const betaBefore = line(app.texts(), "beta");

  await app.press("id:write-beta");
  await app.waitFor("text:/^beta two/");
  expect(line(app.texts(), "beta")).not.toBe(betaBefore);
  expect(line(app.texts(), "alpha")).toBe(alphaBefore);
});

test("a binding on a key wakes for its own writes", async () => {
  const app = await open("../App.tsx", { clock: {} });
  await app.waitFor("text:/^alpha none/");
  const before = line(app.texts(), "alpha");
  await app.press("id:write-alpha");
  await app.waitFor("text:/^alpha one/");
  expect(line(app.texts(), "alpha")).not.toBe(before);
});

test("a write made during a slow read wins over the read's answer", async () => {
  const app = await open("../App.tsx", { clock: {} });
  // The slow read is still in flight: its answer is ten seconds away on the clock.
  expect(line(app.texts(), "gamma").startsWith("gamma none")).toBe(true);
  await app.press("id:write-gamma");
  await app.waitFor("text:/^gamma new/");
  await app.clock!.runFor(10_000);
  expect(line(app.texts(), "gamma").startsWith("gamma new")).toBe(true);
});

test("without a write, a slow read shows the stored value once it ends", async () => {
  const app = await open("../App.tsx", { clock: {} });
  expect(line(app.texts(), "gamma").startsWith("gamma none")).toBe(true);
  await app.clock!.runFor(10_000);
  await app.waitFor("text:/^gamma old/");
});

test("hiding a component ends its listener", async () => {
  const app = await open("../App.tsx", { clock: {} });
  await app.press("id:count-alpha");
  await app.waitFor("text:listeners alpha 1");
  await app.press("id:hide-alpha");
  await app.waitFor("text:hid alpha");
  await app.press("id:count-alpha");
  await app.waitFor("text:listeners alpha 0");
});
