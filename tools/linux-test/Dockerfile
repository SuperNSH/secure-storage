# The Linux test image for nsh-secure-storage: Rust, a D-Bus session and gnome-keyring, so the
# Secret Service tests run for real. Build and run it with ./run.sh.
FROM rust:1-bookworm

# gcc-x86-64-linux-gnu and libc6-dev-amd64-cross let `ring` build for the x86_64 target.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      dbus-daemon dbus-bin dbus-user-session gnome-keyring \
      gcc-x86-64-linux-gnu libc6-dev-amd64-cross \
 && rm -rf /var/lib/apt/lists/* \
 && rustup component add clippy rustfmt \
 && rustup target add x86_64-unknown-linux-gnu

COPY entrypoint.sh /usr/local/bin/linux-test
RUN chmod +x /usr/local/bin/linux-test

# The build folder and the crate cache are volumes (see run.sh), so rebuilds are incremental.
ENV CARGO_TARGET_DIR=/target \
    CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
    CARGO_TERM_COLOR=never
WORKDIR /work
ENTRYPOINT ["/usr/local/bin/linux-test"]
