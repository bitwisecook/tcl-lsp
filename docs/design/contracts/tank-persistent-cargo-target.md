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

The root and target directory must be owned by the runner account, have mode
`700`, and contain no symlink component. The identity marker and lock file are
owned by the runner account, have mode `600`, and are regular, non-symlink
files. The lock also has exactly one hard link. Existing targets are reused
only when the marker matches exactly. Missing, malformed, redirected, or
mismatched state fails closed.

The registration identity comes from `TCL_LSP_TANK_REGISTRATION_ID`, then
the stable `RUNNER_NAME` fallback. `RUNNER_TRACKING_ID` is deliberately not
used: it describes a job/process and can change on every run. Runner images
should set the first value to an immutable registration identifier. A name is
only a compatibility fallback; it must not be shared by two registrations.

## Lifecycle safety

The runner job concurrency group remains `tank`, with `queue: max` and
`cancel-in-progress: false`. Cargo commands hold the target's advisory
`flock`; the bounded janitor examines only old, correctly marked direct
children of the dedicated root. It scans every direct child up to the
`TCL_LSP_TANK_JANITOR_SCAN_LIMIT` ceiling (256 by default), but removes at
most `TCL_LSP_TANK_JANITOR_LIMIT` eligible targets per run. This avoids
starving an old target behind fresh or unmarked entries while bounding a
malicious fan-out. It never waits for a lock, follows a symlink, or removes
an unmarked directory. The free-space floor is checked after janitor work and
before the target is handed to Cargo.

`TCL_LSP_TANK_RETENTION_DAYS`, `TCL_LSP_TANK_JANITOR_LIMIT`,
`TCL_LSP_TANK_JANITOR_SCAN_LIMIT`, and `TCL_LSP_TANK_MIN_FREE_KB` are
explicit, non-negative integer controls. The default floor is 20 GiB. A
malformed control or failed filesystem check is a hard error. The helper
reports `new` versus `reused` state, target bytes, free KiB, and janitor
counts, including when the scan ceiling was reached. Its `report TARGET`
operation emits final size and free-space telemetry after the test attempt.
The existing sccache step reports compiler cache statistics independently;
sccache remains an optimisation and its failure never changes test
correctness.

New targets are assembled in a private `mktemp` directory below the root.
The lock and identity marker are fully validated there before `mv -T`
atomically publishes the identity directory. A private root lock serialises
janitor and creation races. An EXIT trap removes only the exact staging
directory if initialisation fails or the process is interrupted.

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
