#!/usr/bin/env -S bash -eux

export RUSTFLAGS="-Cdebug-assertions=y -Dwarnings"
cargo check --release
<<<<<<< HEAD
=======
cargo check --release --features="bandersnatch-experimental"

export RUSTFLAGS="$RUSTFLAGS --cfg substrate_runtime"
T=wasm32-unknown-unknown
>>>>>>> 07827930 (Use original pr name in prdoc check (#60))
cargo check --release --target=$T --no-default-features
