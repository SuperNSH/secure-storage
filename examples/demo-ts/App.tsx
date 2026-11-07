import { host, mount, Text, View } from "nsh";
import { SecureStorage, SecureStorageError, useSecureValue } from "../../packages/nsh-secure-storage/src";
import { hostBackend } from "../../packages/nsh-secure-storage/src/host";

/** The embedding app's store. Without an embedding host there is nothing to store in, so it says so. */
const storage = new SecureStorage({ backend: hostBackend, iOptions: { accountName: "demo-ts" } });

function codeOf(error: unknown): string {
  return error instanceof SecureStorageError ? error.code : "unknown error";
}

function App() {
  const embedded = host.available();
  const remembered = useSecureValue("remembered", { storage });
  return (
    <View class="flex-1 gap-3 p-4">
      <Text testID="embedded" class="text-foreground">{embedded ? "embedded" : "no host"}</Text>
      <Text testID="state" class="text-foreground">{`remembered ${remembered.state()}`}</Text>
      <Text testID="error" class="text-destructive">{remembered.error()?.code ?? ""}</Text>
      <Text testID="save" role="button" onPress={() => {
        storage.write({ key: "remembered", value: "yes" }).catch((error: unknown) => {
          console.error(codeOf(error));
        });
      }} class="rounded-(--radius) bg-muted px-3 py-2">
        Remember
      </Text>
    </View>
  );
}

mount(App);
