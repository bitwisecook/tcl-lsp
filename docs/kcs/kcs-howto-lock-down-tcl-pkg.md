# KCS: How do I lock down `tcl pkg` for my organisation?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI (`tcl pkg`), the Rust package manager.

## Question

I administer developer machines and want `tcl pkg` to enforce a security floor
that developers cannot weaken: confine anything it runs, restrict where packages
come from, scan packages as they are fetched, and keep package build scripts from
running unless I approve them. How do I deploy that?

## Before you start

- Administrative (root / Administrator) access to the machines you manage.
- The package manager already separates *what runs* from *who allows it*: every
  external command runs in a sandbox, and policy is merged from three layers —
  **system** (operator), **user**, then **project**. Only the system layer can
  *lock* settings. See [`tclpkg-security.md`](../design/tclpkg-security.md) for
  the full design.

## Answer

### 1. Write the system policy

Create the operator policy file. It must be owned by root/Administrators and not
world-writable, or it is ignored (so a developer cannot replace it with their
own).

- POSIX: `/etc/tcl-lsp/pkg-policy.toml`
- Windows: `%PROGRAMDATA%\tcl-lsp\pkg-policy.toml`

```toml
# Sandbox floor applied to everything tcl pkg runs.
[sandbox]
fail-closed = true            # refuse to run if the floor can't be enforced
require-network-deny = true   # default-deny executions must really have no network
max-timeout-secs = 300
env-deny = ["GITHUB_TOKEN", "NPM_TOKEN"]   # belt-and-braces; secrets are stripped anyway

# Only fetch packages from the corporate mirror, over TLS.
[registry]
require-https = true
allow = ["https://packages.corp.example/"]

# Don't use a version until it has been public for a week (catches worm
# replication waves and fresh typosquats); surfaced to your cooldown hook.
[cooldown]
min-release-age-days = 7

# Refuse to lock a package that has no integrity hash.
[verification]
require-integrity = true

# Packages are data: build scripts stay off until you both flip this and
# trust the specific package.
[build]
allow-build-scripts = false

# Run a scanner over every fetched package; a non-zero exit aborts the install.
[[hooks]]
name = "scan-fetched"
stage = "post-fetch"
command = ["/opt/secscan/tcl-scan", "${TCLPKG_PKG_DIR}"]
network = false
timeout-secs = 60

# Freeze all of the above so user/project config can't loosen it.
lock = ["sandbox", "registry", "cooldown", "verification", "build", "hooks"]
```

### 2. Verify it is being honoured

```console
$ tcl pkg policy show
Policy layers (low → high precedence):
  system   /etc/tcl-lsp/pkg-policy.toml [loaded]
  user     ~/.config/tcl-lsp/pkg.toml  [loaded]
...
Admin-locked keys:
  build (section)
  registry (section)
  sandbox (section)
  ...
```

If the system file shows `[—] (ignored: …)` it is not root-owned or is
world-writable — fix the ownership/permissions. Use `tcl pkg policy verify` in
your fleet tooling; it exits non-zero when anything is wrong (untrusted system
file, schema error, or a lower layer trying to override a locked key).

### 3. What developers now experience

- Installs from anywhere other than the corporate mirror are rejected.
- Every fetched package is scanned; a finding aborts the install.
- A package that ships a build script does nothing on install. If a developer
  needs it, *you* enable it: set `allow-build-scripts = true` and add the
  package to `[build] trusted` (or have the developer run `tcl pkg trust <pkg>`
  if you did not lock the `build` section). It then runs deprivileged — no
  ambient secrets, a throwaway `HOME`, and no network unless the manifest
  declared it and policy allows it.
- A user `~/.config/tcl-lsp/pkg.toml` or project `tclpkg.toml` may still set
  *unlocked* keys, but any attempt to override a locked one is ignored and
  reported.

### 4. Audit what ran

Every sandboxed execution is logged:

```console
$ tcl pkg audit --lines 20
{"ts":"…","label":"hook:post-fetch:scan-fetched","isolation":"baseline","success":true,…}
{"ts":"…","label":"git-clone","network_enforced":true,"success":true,…}
```

## Notes

- The sandbox always applies a portable baseline (environment scrubbed of
  credentials, working-directory confinement, timeouts). Stronger OS-native
  confinement (Landlock/seccomp, Seatbelt, `pledge`/`unveil`, Capsicum, Job
  Objects) is layered on where available; where it is not, `fail-closed`
  policies refuse rather than run under-confined, and commands report the
  isolation level they achieved.
- Hooks receive context as `TCLPKG_*` environment variables
  (`TCLPKG_NAME`, `TCLPKG_VERSION`, `TCLPKG_SOURCE_URL`, `TCLPKG_INTEGRITY`,
  `TCLPKG_PKG_DIR`); reference them in a hook `command` with `${TCLPKG_…}`.
