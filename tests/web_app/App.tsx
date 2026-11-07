// The web test app: each button runs one store call and shows its outcome as text, so the
// headless browser test (App.test.tsx) can drive it with presses and read what it shows.
import { mount, signal, Text, View } from "nsh";
import { NativeSecureStorage } from "nsh-secure-storage/native";

const store = new NativeSecureStorage();

function errorText(error: unknown): string {
  const code = (error as { code?: string }).code;
  return `error ${code ?? "unknown"}`;
}

function App() {
  const status = signal("idle");

  async function run(action: () => Promise<string>) {
    try {
      status.set(await action());
    } catch (error) {
      status.set(errorText(error));
    }
  }

  const buttons: [string, () => Promise<string>][] = [
    ["Write", async () => { await store.write("token", "s3cret-test-value"); return "written"; }],
    ["Read", async () => `read [${(await store.read("token")) ?? "none"}]`],
    ["Contains", async () => `contains ${await store.containsKey("token")}`],
    ["Delete", async () => { await store.delete("token"); return "deleted"; }],
    ["List", async () => {
      const entries = await store.readAll();
      return `list [${entries.map((e) => e.key).join(",")}]`;
    }],
    ["Delete all", async () => { await store.deleteAll(); return "cleared"; }],
    ["Write three", async () => {
      await Promise.all(["a", "b", "c"].map((k) => store.write(k, `v-${k}`)));
      return "three written";
    }],
    ["Write empty", async () => { await store.write("token", ""); return "empty written"; }],
    ["Write nul", async () => { await store.write("bad\u0000key", "x"); return "nul written"; }],
  ];

  return (
    <View class="flex-1 items-center justify-center gap-3 bg-background p-4">
      <Text testID="status" class="text-lg text-foreground">{status()}</Text>
      {buttons.map(([label, action]) => (
        <View
          onPress={() => run(action)}
          role="button"
          class="rounded-(--radius) bg-primary px-5 py-3"
        >
          <Text class="text-primary-foreground">{label}</Text>
        </View>
      ))}
    </View>
  );
}

mount(App);
