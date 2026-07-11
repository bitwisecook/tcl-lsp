# RUST_ISSUE_007: ≥13 registry Tcl commands have no runtime handler and no not-required classification, so they error under the WASM/tree-walking path

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `registry↔runtime` |
| **Status** | Resolved |
| **Verification** | Verified firsthand by reviewer |

## Finding

registry↔runtime — ≥13 registry Tcl commands have no runtime handler and no not-required classification, so they error under the WASM/tree-walking path.
Absent from the 112 `register_builtin` names: `exec, exit, time, timerate, tailcall, lpop, lremove, zlib, pid, fileevent, fcopy, load, socket` — each has a registry spec. `exec/exit/socket/load` appear only in the safe-interp hide list. A miss routes to `unknown` → "invalid command name". Compute-only ones (tailcall, lpop, lremove, time, timerate) have no portability excuse. `exit 0` doesn't exit; `time {...}`/`tailcall f`/`lpop l` raise "invalid command name". Confidence: high

## Progress

The gap is now **measured and gated** rather than silent: `RUST_ISSUE_006`'s
`cargo xtask command-backing` gate classifies every core command and lists the
residue in `docs/generated/wasm-command-backing.md`. Of the original list:

- **Backed** (`handler`): `lpop`, `lremove`, `pid` — the compute-only list commands
  and `pid` now have real `register_builtin` handlers.
- **Explicit stubs** (`not-required`): `exec`, `socket`, `load`, `fileevent`, `fcopy`
  are loop-registered as "not supported under the WASM runtime" errors (external
  process / socket / native load / event loop — the portability excuse), so they no
  longer route to `unknown`.
- **Now backed** (`handler`): every remaining name has gained a real
  `register_builtin` handler, so the gate's `KNOWN_UNBACKED` allow-list is **empty**
  and `cargo xtask command-backing` reports `UNCLASSIFIED=0` with no known gaps:
  - `exit` records the code and unwinds uncatchably (it must **not** kill the
    embedding process); `catch` re-propagates while it is set.
  - `time` / `timerate` measure over the host clock; `timerate` reproduces C's
    8-word `µs/# … net-ms` report, `-overhead`/`-calibrate`, and break/continue
    semantics.
  - `tailcall` dispatches the call and carries its result out of the proc.
  - `chan` forwards the supported subcommands to the existing channel handlers and
    reports the event-driven / reflected / stacked-channel ones as unsupported.
  - `::tcl::unsupported::corotype` classifies a coroutine as `active`/`yield`.
  - `classvariable` links a method-local to its declaring class's namespace.
  - `coroprobe` / `coroinject` extend the coroutine worker's rendezvous protocol
    to run a command in — or queue one for — a suspended coroutine's context.
  - `zlib` implements the checksums natively and drives DEFLATE/zlib/gzip through
    `flate2`'s pure-Rust `miniz_oxide` backend (wasm-clean, no C `libz`).

**Resolved:** every core registry command now has a runtime handler or an explicit
not-required / stdlib classification; the `RUST_ISSUE_006` gate enforces this with
an empty `KNOWN_UNBACKED`.
