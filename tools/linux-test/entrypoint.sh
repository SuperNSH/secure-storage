#!/bin/sh
# Runs each command given as arguments inside a private D-Bus session with an unlocked
# gnome-keyring. The keyring password is a throwaway that exists only in this container.
set -eu

export KEYRING_PASSWORD=throwaway-test-password
export NSH_SECURE_STORAGE_LINUX_TESTS=1

# One session per command: the locked-keyring test locks the keyring for its own session only.
for command in "$@"; do
    dbus-run-session -- sh -c '
        printf "%s" "$KEYRING_PASSWORD" | gnome-keyring-daemon --unlock --components=secrets >/dev/null
        eval "$1"
    ' sh "$command"
done
