# `tcl pkg` security architecture: sandboxing, operator hooks, locked-down policy

Companion to [`architecture.md`](architecture.md). This document
describes the security model of the package manager — `rust/tcl-pkg`,
`rust/tcl-cli`, and `rust/tcl-sandbox`.

## Goals

1. **Everything `tcl pkg` runs is deprivileged.** `git`, `tclsh`, operator
   hooks and — above all — package-provided build scripts run through one
   sandbox that strips ambient authority (credentials in the environment, the
   user's home directory, unrestricted network) instead of inheriting it.
2. **Operators, not developers, set security policy** — in a file developers
   cannot edit, with the ability to **lock** individual settings so lower
   configuration layers cannot loosen them.
3. **Packages are data by default.** No package-provided code ever runs as a
   side effect of resolving or installing. A build phase exists only as an
   explicit, operator-gated, sandboxed opt-in.

## Threat model

The design is grounded in real supply-chain attacks. The dominant vector across
every ecosystem is **arbitrary code execution at build/install time** with full
user privilege and ambient secrets/network:

| Ecosystem | Representative incidents | Execution vector |
|---|---|---|
| npm | event-stream (2018), ua-parser-js (2021), coa/rc (2021), node-ipc (2022), chalk/debug (Sept 2025), Shai-Hulud worm (2025) | `preinstall`/`postinstall` lifecycle scripts run with full user privilege during `npm install` |
| PyPI | `colourama`/`jeIlyfish` typosquats, W4SP stealer, `ultralytics` CI cache poisoning (2024) | `setup.py` executes arbitrary code when installing an sdist |
| **cargo** | `rustdecimal` (2022), `faster_log`/`chrono_anchor` (2025-26) | **`build.rs` and proc-macros run arbitrary code at build time** — the closest precedent for us |
| AUR | Atomic Arch (2026) | `PKGBUILD` `.install` scripts run as root |
| xz-utils | CVE-2024-3094 (2024) | multi-year maintainer social engineering; payload hidden in a release tarball and injected by an `m4` macro **at build time**, bypassing source review |

Secondary vectors: resolve-time (dependency confusion, typosquatting),
fetch-time (cache poisoning), and publish/maintainer-account compromise. The
TanStack compromise (2026) shipped malware with **valid SLSA provenance** —
proof that provenance attests *who built it*, not *that it is safe*.

### Defence mapping

No single control suffices; the design layers them. Each row is a control and
where it lives in the implementation.

| Control | Mitigates | Where |
|---|---|---|
| Packages are pure data; manifest is parse-only | install/postinstall RCE | `manifest.rs` (whitelisted directives, no interpreter) |
| Deprivileged build sandbox (no secrets, no net, confined FS) | `build.rs`/`setup.py`-style build RCE, xz-style build injection | `tcl-sandbox`, `tcl pkg build` |
| Environment scrubbing | credential theft from env (event-stream, Shai-Hulud) | `tcl-sandbox` baseline |
| Operator hooks (scan/deny at lifecycle stages) | malware detection, org policy, cooldown, provenance | `hooks.rs` |
| Admin-locked policy (registry allow/deny, require-https, required signature) | dependency confusion, untrusted registries | `policy.rs` |
| Integrity hashes + lockfile pinning | tarball swap, cache poisoning | `cas.rs`, `lockfile.rs`, `verification.require-integrity` |
| Cooldown / min-release-age | worm replication window, fresh typosquats | `cooldown` policy (surfaced to hooks) |
| Network egress denial during build | build-time exfiltration | sandbox `deny-network` / per-profile network grant |

Provenance/signature verification is supported only as an optional hook, never
as the sole gate, because of the TanStack lesson. A Go-style transparency log is
registry-side infrastructure and out of scope for the client.

## Architecture

```
                       ┌──────────────────────────────┐
   tclpkg.tcl  ──────► │ manifest.rs (parse-only,      │
   (pure data)         │ whitelisted directives)       │
                       └──────────────────────────────┘
   /etc/…/pkg-policy.toml ┐
   ~/.config/…/pkg.toml   ├─► policy.rs ─► PolicyConfig ─┐
   ./tclpkg.toml          ┘   (layered + locked)         │
                                                         ▼
   git / tclsh / hooks / build script ──► exec.rs ──► tcl-sandbox::run
                                          (chokepoint    (capability ∩ policy,
                                           + audit log)   baseline + OS tier)
```

### The sandbox crate (`rust/tcl-sandbox`)

A standalone, `unsafe`-free crate. A [`Profile`] describes what a command
*requests* (program, args, cwd, env passthrough, network, fs read/write,
timeout); a [`SandboxPolicy`] is the operator floor. `run()` computes the
effective grant (`requested`, clamped and widened by the floor) and executes the
child under the strongest available **confinement tier**:

- **Baseline (every platform, always applied):** `env_clear()` + an explicit
  allow-list with a sensitive-name denylist (`*_TOKEN`, `AWS_*`, `SSH_*`,
  `GITHUB_*`, `NETRC`, …), working-directory pinning, output capture, and a
  wall-clock timeout that kills runaways.
- **OS-native (Linux):** behind the [`Confinement`] trait, Landlock confines a
  profile that names any `fs_read`/`fs_write` path to them — read and execute
  beneath the read paths and the working directory, everything beneath the
  write paths, plus read access to the system trees (`/usr`, `/lib*`, `/bin`,
  `/sbin`, `/etc`, `/opt`, `/nix/store`), the program's own directory and the
  `lib`/`share` beside it, and the standard devices. A profile denied the
  network gets a seccomp filter refusing `socket` for `AF_INET`, `AF_INET6` and
  `AF_PACKET` (and `io_uring_setup`, which could open one past it) with
  `EACCES`. Both restrict the calling thread and what it forks, so the child
  is spawned from a thread of its own that takes them on first; the parent
  keeps its authority. Each mechanism is probed once per process on a
  throwaway thread, so a kernel without it reports Baseline for it rather
  than claiming it.
- **Other platforms:** Seatbelt on macOS, `pledge`/`unveil` on OpenBSD,
  Capsicum on FreeBSD and restricted tokens + Job Objects on Windows are not
  built; `detect_confinement()` answers `Baseline` there.

`Outcome::isolation` is `Os` only when the OS layer enforced something for that
run, and `Outcome::enforced` says which (`network`, `filesystem`); the audit log
records both, `tcl pkg build` warns when either was not enforced, and a
`fail-closed` policy refuses rather than under-confine.

If a policy marks a floor mandatory (e.g. `require-network-deny`) and the host
cannot enforce it, `run()` **fails closed** rather than running weaker than
promised. The crate writes no `unsafe`, which the workspace
`unsafe_code = "forbid"` lint holds it to.

### Layered, lockable policy (`rust/tcl-pkg/src/policy.rs`)

TOML, merged lowest-precedence first:

1. **System** — `/etc/tcl-lsp/pkg-policy.toml` (POSIX) /
   `%PROGRAMDATA%\tcl-lsp\pkg-policy.toml` (Windows). Honoured only when the
   file is owned by root/Administrators and not world-writable, so a developer
   cannot weaken it by editing a file they own.
2. **User** — `~/.config/tcl-lsp/pkg.toml`.
3. **Project** — `tclpkg.toml` beside the manifest.

The system layer can **lock** keys:

- `lock = ["sandbox.deny-network", "registry"]` — a dotted leaf is frozen at its
  system value; a whole section name freezes the entire subtree (no sibling
  additions).
- `lock-all = true` — every leaf the system layer sets is frozen, while still
  letting lower layers add keys the system did not set.

Override attempts against a locked key are ignored and surfaced as warnings
(`tcl pkg policy show` / `verify`).

#### Schema

```toml
[sandbox]
fail-closed = false            # abort when a mandatory floor can't be met
require-network-deny = false   # network denial must be enforceable
deny-network = false           # force network off for everything
max-timeout-secs = 300         # clamp every command's timeout
env-allow = ["PATH"]           # extra passthrough names
env-deny  = []                 # names never passed through

[registry]
allow = ["https://reg.corp/"]  # if non-empty, only these source prefixes
deny  = []
require-https = true           # reject http:// sources

[cooldown]
min-release-age-days = 7       # surfaced to hooks for enforcement

[verification]
require-integrity = true       # install fails if any package lacks a hash
require-provenance = false     # delegated to a verification hook

[build]
allow-build-scripts = false    # packages are data unless an operator opts in
trusted = []                   # packages whose build script may run

[[hooks]]
name = "scan-fetched"
stage = "post-fetch"
command = ["/opt/secscan/tcl-scan", "${TCLPKG_PKG_DIR}"]
network = false
timeout-secs = 30

lock = ["sandbox", "registry", "verification", "build", "hooks"]
```

### The execution chokepoint (`exec.rs`)

Nothing in `tcl-pkg` spawns a process directly. `exec::execute()` builds the
profile, applies the sandbox policy and appends a JSON audit record (label,
program, requested/enforced network, enforced filesystem, achieved isolation,
exit code, timeout) to
`$XDG_STATE_HOME/tcl-lsp/pkg-audit.log` (`tcl pkg audit`). The wired call sites:

- `fetchers::fetch_git` — git clone/rev-parse; the one trusted tool granted the
  network, still with credentials scrubbed (only PATH/HOME/proxy/TLS pass).
- `venv::tclsh_version_string` — the tclsh version probe (script on stdin).
- `tcl pkg run` — the project's own entry point (full env, network allowed, but
  audited, policy-clamped and wrapped in pre/post-run hooks).
- `tcl pkg build` — the deprivileged build script (see below).

### Operator hooks (`hooks.rs`)

Hooks are defined **in policy** (so they live in the locked-down system layer)
and fire at lifecycle stages: `pre/post-resolve`, `pre/post-fetch`,
`pre/post-build`, `pre/post-install`, `pre/post-run`. Each hook runs through the
sandbox with its declared, policy-clamped capabilities. Context (package name,
version, source URL, integrity, on-disk path) is passed as scrubbed `TCLPKG_*`
environment variables, with `${VAR}` expansion in the command and path entries.
**A non-zero exit aborts the operation** — this is how an operator's scanner,
denylist, cooldown check or provenance verifier blocks a package.

### Build phase (`build` directive + `tcl pkg build`)

The manifest gains a data-only `build` directive:

```tcl
build  build.tcl           ;# script path, relative to the manifest
build  build.tcl -network  ;# optionally declares it needs the network
```

Declaring it never causes execution. `tcl pkg build` runs the script **only**
when policy both enables build scripts (`[build] allow-build-scripts = true`) and
trusts the package (`[build] trusted` / `tcl pkg trust <name>`) — mirroring npm
v12 / pnpm / Bun / Deno's "scripts off by default". When it does run, it is the
most confined profile: environment scrubbed of secrets, `HOME` pointed at a
throwaway directory, network denied unless the manifest declared it (and policy
allows it), its filesystem confined to the project tree, the temporary directory
and the Tcl libraries named by `TCL_LIBRARY`/`TCLLIBPATH` where the host
enforces it, with the achieved isolation level reported.

## CLI surface

| Command | Purpose |
|---|---|
| `tcl pkg policy show [--json]` | Print the merged policy, its layers, and which keys are admin-locked |
| `tcl pkg policy verify [--json]` | Validate the policy files; non-zero on warnings (untrusted system file, schema errors, ignored overrides) |
| `tcl pkg hooks [--json]` | List the operator hooks bound to each stage |
| `tcl pkg audit [--lines N] [--json]` | Show recent sandboxed-execution audit records |
| `tcl pkg trust <pkg> [--remove]` | Add/remove a package from the per-user build-trust list |
| `tcl pkg build` | Run the manifest's declared build script, deprivileged |

## Testing

- `tcl-sandbox` unit tests cover env scrubbing (including the sensitive-name
  denylist), the fail-closed path, the timeout kill, capability merging, and —
  on Linux — that a granted path is reachable and an ungranted one is not, and
  that a child denied the network cannot open an internet socket (each
  asserting the outcome reports what the kernel actually enforced),
  without mutating the process environment (an injectable env lookup keeps the
  crate `unsafe`-free under edition 2024).
- `tcl-pkg` unit tests cover layered policy merging and all three lock modes
  (locked leaf, locked section, `lock-all`), the registry allow/deny gate, and
  hook stage parsing / `${VAR}` expansion.
- End-to-end: an operator hook that exits non-zero aborts `tcl pkg install`; a
  `build` script cannot see a `GITHUB_TOKEN` planted in the parent environment
  and runs against a throwaway `HOME`; `tcl pkg build` still builds under the
  confinement, and a script that did not declare the network cannot reach a
  local listener wherever the audit log says the denial was enforced
  (`rust/tcl-cli/tests/pkg_build.rs`).

## Boundaries of the model

These are the edges of what the sandbox promises, and they matter when
reasoning about a threat:

- **OS-native isolation is Linux-only, and partial there.** Elsewhere the
  enforced confinement is environment scrubbing, cwd pinning, output capture
  and the wall-clock timeout. On Linux, Landlock does not govern every
  operation (sockets, signals, `ioctl` on already-open files, and on older
  kernels truncation and renames across directories), and the seccomp filter
  refuses internet sockets but not a local socket that a daemon outside the
  sandbox could forward; a profile that names no path is not filesystem
  confined at all. A policy that needs more must set `fail-closed`.
- **Only the project's own build script runs.** The resolver does not read
  a dependency's manifest for a declared build phase, so a dependency's
  `build` directive is inert.
- **No child resource limits.** Memory and CPU rlimits are not applied; a
  runaway is bounded by the timeout, not by consumption.
- **No transparency log.** Verifying that a registry has not equivocated
  is registry-side infrastructure, outside the client's reach; the client
  offers integrity hashes, lockfile pinning, and provenance hooks
  instead.
- **Spec-pack hooks are a different sandbox.** A `.tclspec` pack's analysis
  hooks run in the language server, not in `tcl pkg`, under their own
  closed-whitelist VM and budgets. That model — and why the workspace pack
  tier loads in an untrusted workspace while `tclLsp.specPacks` does not —
  is in [`spec-packs.md`](../registry/spec-packs.md), "Covering the hooks" and
  "Workspace trust".
