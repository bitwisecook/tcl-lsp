# Persistent Cargo targets on Tank

This contract defines the retained Cargo target used by the trusted
self-hosted `tank` runner. Hosted overflow stays ephemeral.

## Identity and layout

`persistent-cargo-target.sh prepare tank` accepts the runner registration,
repository, and canonical checkout root. It hashes those three values and
uses the digest as the directory name beneath the dedicated target root:

```text
/home/runner/.cache/tcl-lsp/cargo-targets/<identity-sha256>/
```

The root, target directory, and marker must be owned by the runner account,
have mode `700`, and contain no symlink component. The marker has mode `600`
and records all identity values. Existing targets are reused only when the
marker matches exactly. Missing, malformed, redirected, or mismatched state
fails closed.

The registration identity comes from `TCL_LSP_TANK_REGISTRATION_ID`, then
`RUNNER_TRACKING_ID`, then `RUNNER_NAME`. Runner images should set the first
value to an immutable registration identifier. A name is only a compatibility
fallback; it must not be shared by two registrations.

## Lifecycle safety

The runner job concurrency group remains `tank`, with `queue: max` and
`cancel-in-progress: false`. Cargo commands hold the target's advisory
`flock`; the bounded janitor examines only old, correctly marked direct
children of the dedicated root. It never waits for a lock, follows a
symlink, or removes an unmarked directory. The free-space floor is checked
after janitor work and before the target is handed to Cargo.

`TCL_LSP_TANK_RETENTION_DAYS`, `TCL_LSP_TANK_JANITOR_LIMIT`, and
`TCL_LSP_TANK_MIN_FREE_KB` are explicit, non-negative integer controls. The
default floor is 10 GiB. A malformed control or failed filesystem check is a
hard error. The helper reports `new` versus `reused` state, target bytes,
free KiB, and janitor counts. The existing sccache step reports compiler
cache statistics independently; sccache remains an optimisation and its
failure never changes test correctness.

## Hosted and policy boundaries

Hosted overflow does not run the helper and does not set `CARGO_TARGET_DIR`.
The workflow path classifier treats the helper, its contract test, and the
runner workflow as runner-policy inputs. Such changes force hosted proof
before trusted Tank execution. `cache-targets: false` remains in place:
persistent local targets provide reuse while the Actions target archive stays
disabled.

The executable contract is tested by
[`test-persistent-cargo-target.sh`](../../../scripts/dev/test-persistent-cargo-target.sh)
and included in `make xtask-check`.
