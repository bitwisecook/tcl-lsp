# How is the omitted Wasmtime tool obtained and verified locally?

The stored executable included public vendor trust-anchor key material. The
compressed executable is now an explicit redaction notice. It cannot be used
as the original executable. Historical execution receipts and the original
uncompressed SHA-256 in `selected-input-storage.json` remain historical
identity evidence, not a claim that executable bytes are still retained.

Obtain the tool outside the repository, then compare it with the recorded
original hash and version before attempting a new measurement:

```sh
umask 077
RUNNER_DIR=$(mktemp -d /tmp/tcl-wasmtime.XXXXXX)
trap 'rm -rf "$RUNNER_DIR"' EXIT
curl --fail --location --silent --show-error \
  https://github.com/bytecodealliance/wasmtime/releases/download/v49.0.2/wasmtime-v49.0.2-x86_64-linux.tar.xz \
  -o "$RUNNER_DIR/runner.tar.xz"
tar -xJf "$RUNNER_DIR/runner.tar.xz" -C "$RUNNER_DIR"
RUNNER="$RUNNER_DIR/wasmtime-v49.0.2-x86_64-linux/wasmtime"
actual=$(openssl dgst -sha256 "$RUNNER" | awk '{print $NF}')
expected=$(jq -r '.inputs[] | select(.original_path | endswith("/wasmtime")) | .original_sha256' \
  rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-input-storage.json)
test "$actual" = "$expected"
"$RUNNER" --version
```

A mismatch means the downloaded tool is a different input. Stop the original
replay and retain a new measurement with its own provenance instead. Obtaining
this tool does not reconstruct the original environment or dependency closure.
Keep the downloaded executable and its embedded trust material outside the
checkout. The broader [material recipes](../../../../../../../experiments/name-handling/results/security-material-scrub/material-generation.md)
cover local certificate creation and OpenSSL verification.
