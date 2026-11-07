#!/bin/sh
# Builds the Linux test image and runs the Secret Service tests in it. The repository is mounted
# at /work; the build folder and the crate cache are Docker volumes, so repeat runs are quick.
#
# Usage: tools/linux-test/run.sh ['cargo command' ...]
# With no arguments it runs the Secret Service suite, the locked keyring and the no-service test.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)

docker build -q -t nsh-secure-storage-linux-test -f "$HERE/Dockerfile" "$HERE" >/dev/null

if [ "$#" -eq 0 ]; then
    set -- \
        "timeout 300 cargo test --no-fail-fast -p nsh-secure-storage --all-features --test linux --test linux_secret_service --test linux_no_service" \
        "timeout 300 cargo test -p nsh-secure-storage --all-features --test linux_locked_keyring"
fi

exec docker run --rm \
    -v "$REPO":/work \
    -v nsh-secure-storage-target:/target \
    -v nsh-secure-storage-cargo:/usr/local/cargo/registry \
    nsh-secure-storage-linux-test "$@"
