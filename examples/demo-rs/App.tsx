import { host, mount, signal, Text, View } from "nsh";
import { SecureStorage, SecureStorageError, useSecureValue } from "../../packages/nsh-secure-storage/src";
import { rustBackend } from "../../packages/nsh-secure-storage/src/rust";

/** A file keychain when the host names one (tests and development); the app's keychain otherwise. */
const keychainPath = typeof host.params?.keychainPath === "string" ? host.params.keychainPath : undefined;

const storage = new SecureStorage({
  backend: rustBackend,
  mOptions: keychainPath === undefined ? {} : { usesDataProtectionKeychain: false, keychainPath },
});

const status = signal("idle");

function codeOf(error: unknown): string {
  return error instanceof SecureStorageError ? error.code : "unknown error";
}

function App() {
  const token = useSecureValue("token", { storage });
  const run = (action: () => Promise<string>) => {
    action().then((text) => status.set(text), (error: unknown) => status.set(codeOf(error)));
  };
  return (
    <View class="flex-1 gap-3 p-4">
      <Text testID="status" class="text-foreground">{status()}</Text>
      <Text testID="token" class="text-foreground">{`token ${token.state()}`}</Text>
      <Text testID="save" role="button" onPress={() => run(() => storage.write({ key: "token", value: "demo-value" }).then(() => "saved"))} class="rounded-(--radius) bg-muted px-3 py-2">
        Save
      </Text>
      <Text testID="load" role="button" onPress={() => run(() => storage.read({ key: "token" }).then((v) => (v === null ? "absent" : "present")))} class="rounded-(--radius) bg-muted px-3 py-2">
        Load
      </Text>
      <Text testID="remove" role="button" onPress={() => run(() => storage.delete({ key: "token" }).then(() => "removed"))} class="rounded-(--radius) bg-muted px-3 py-2">
        Remove
      </Text>
    </View>
  );
}

mount(App);
