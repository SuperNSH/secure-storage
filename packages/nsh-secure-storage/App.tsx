import { host, mount, signal, Text, View } from "nsh";
import { hostBackend } from "./src/host";
import { memoryBackend } from "./src/memory";
import { SecureStorage, SecureStorageError, useSecureValue } from "./src";

/** The embedding app's store when there is one, else a memory store. */
const storage = host.available()
  ? new SecureStorage({ backend: hostBackend, iOptions: { accountName: "nsh-secure-storage-demo" } })
  : SecureStorage.memory();

/** Its reads take ten seconds on the clock, so a write can land while one is in flight. */
const slow = new SecureStorage({
  backend: memoryBackend({ initialValues: { gamma: "old" }, readDelayMs: 10_000 }),
});

const status = signal("idle");
const heard = signal("nothing heard");
const shown = signal(true);

function codeOf(error: unknown): string {
  return error instanceof SecureStorageError ? error.code : "unknown error";
}

/** A button. It shows what its action returns, or the error's code. */
function Action(props: { id: string; label: string; run: () => Promise<string> }) {
  const press = () => {
    props.run().then(
      (text) => status.set(text),
      (error: unknown) => status.set(codeOf(error)),
    );
  };
  return (
    <Text testID={props.id} role="button" onPress={press} class="rounded-(--radius) bg-muted px-3 py-2">
      {props.label}
    </Text>
  );
}

/** A key as a reactive value. Its line counts the runs of its own binding. */
function Probe(props: { name: string; store: SecureStorage }) {
  const secure = useSecureValue(props.name, { storage: props.store });
  let runs = 0;
  const line = () => {
    runs += 1;
    return `${props.name} ${secure.value() ?? "none"} runs ${runs}`;
  };
  return <Text testID={`probe-${props.name}`} class="text-foreground">{line()}</Text>;
}

function App() {
  return (
    <View class="flex-1 gap-3 p-4">
      <Text testID="status" class="text-foreground">{status()}</Text>
      <Text testID="heard" class="text-foreground">{heard()}</Text>
      <View class="flex-row flex-wrap gap-2">
        <Action id="write-alpha" label="Write alpha" run={() => storage.write({ key: "alpha", value: "one" }).then(() => "wrote alpha")} />
        <Action id="write-beta" label="Write beta" run={() => storage.write({ key: "beta", value: "two" }).then(() => "wrote beta")} />
        <Action id="read-alpha" label="Read alpha" run={() => storage.read({ key: "alpha" }).then((v) => `read alpha ${v === null ? "absent" : "present"}`)} />
        <Action id="contains-alpha" label="Contains alpha" run={() => storage.containsKey({ key: "alpha" }).then((p) => `contains alpha ${p}`)} />
        <Action id="delete-alpha" label="Delete alpha" run={() => storage.delete({ key: "alpha" }).then(() => "deleted alpha")} />
        <Action id="read-all" label="Read all" run={() => storage.readAll().then((all) => `entries ${all.length}`)} />
        <Action id="delete-all" label="Delete all" run={() => storage.deleteAll().then(() => "deleted all")} />
        <Action id="bad-key" label="Bad key" run={() => storage.read({ key: "" }).then(() => "read ok")} />
        <Action id="listen" label="Listen beta" run={() => {
          storage.registerListener({
            key: "beta",
            listener: (value) => heard.set(value === null ? "heard beta removed" : "heard beta written"),
          });
          return Promise.resolve("listening to beta");
        }} />
        <Action id="count-alpha" label="Count alpha listeners" run={() => Promise.resolve(`listeners alpha ${storage.listenerCount("alpha")}`)} />
        <Action id="hide-alpha" label="Hide alpha" run={() => {
          shown.set(false);
          return Promise.resolve("hid alpha");
        }} />
        <Action id="write-gamma" label="Write gamma" run={() => slow.write({ key: "gamma", value: "new" }).then(() => "wrote gamma")} />
        <Action id="protected" label="Protected data" run={() => storage.isCupertinoProtectedDataAvailable().then((a) => `protected ${a}`)} />
      </View>
      <If when={shown()}>
        <Probe name="alpha" store={storage} />
      </If>
      <Probe name="beta" store={storage} />
      <Probe name="gamma" store={slow} />
    </View>
  );
}

mount(App);
