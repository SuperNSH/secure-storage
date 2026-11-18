#!/bin/sh
# Rebuilds native/android/classes.dex from the Java source. The output is the same on every run
# with the same JDK and build-tools. Usage: build-dex.sh [output]
# Needs ANDROID_HOME, a JDK, and the build-tools named by BUILD_TOOLS (default: the newest).
set -eu

here=$(cd "$(dirname "$0")" && pwd)
sdk=${ANDROID_HOME:-${ANDROID_SDK_ROOT:?set ANDROID_HOME}}
platform=$(ls -d "$sdk"/platforms/android-* | sort -V | tail -n 1)
if [ -n "${BUILD_TOOLS:-}" ]; then
  tools="$sdk/build-tools/$BUILD_TOOLS"
else
  tools=$(ls -d "$sdk"/build-tools/* | sort -V | tail -n 1)
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

javac --release 8 -Xlint:-options -cp "$platform/android.jar" -d "$work/classes" \
  "$here"/java/dev/nsh/securestorage/*.java
# Sorted, so the dex does not depend on directory order.
classes=$(find "$work/classes" -name '*.class' | LC_ALL=C sort)
"$tools/d8" --release --min-api 28 --lib "$platform/android.jar" --output "$work" $classes
cp "$work/classes.dex" "${1:-$here/classes.dex}"
