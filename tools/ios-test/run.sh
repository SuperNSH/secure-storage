#!/bin/sh
# Runs tests/ios_keychain.rs on a booted simulator, against its real Keychain.
# The simulator reads an app's entitlements from these Mach-O sections, as Xcode's Simulated.xcent does.
# Usage: tools/ios-test/run.sh <simulator udid>
set -eu
udid=$1
root=$(cd "$(dirname "$0")/../.." && pwd)
work=${CARGO_TARGET_DIR:-$root/target}/ios-test
mkdir -p "$work"
derq query -f xml -i "$root/tools/ios-test/test.xcent" -o "$work/test.der" --raw
export RUSTFLAGS="-C link-arg=-Wl,-sectcreate,__TEXT,__entitlements,$root/tools/ios-test/test.xcent -C link-arg=-Wl,-sectcreate,__TEXT,__ents_der,$work/test.der"
binary=$(cargo test -p nsh-secure-storage --target aarch64-apple-ios-sim --test ios_keychain --no-run --message-format=json \
  | python3 -I -c 'import json,sys
for line in sys.stdin:
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m.get("executable") and "ios_keychain" in m["target"]["name"]:
        print(m["executable"])')
xcrun simctl spawn "$udid" "$binary" --test-threads=1
