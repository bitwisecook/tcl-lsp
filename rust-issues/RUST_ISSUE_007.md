# RUST_ISSUE_007: ≥13 registry Tcl commands have no runtime handler and no not-required classification, so they error under the WASM/tree-walking path

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `registry↔runtime` |
| **Status** | Open |
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
- **Still unimplemented** (`known-gap`, allow-listed so the `RUST_ISSUE_006` gate
  stays green while they land one by one): `exit`, `time`, `timerate`, `tailcall`,
  `zlib`, plus `chan`, `coroinject`, `coroprobe`, `::tcl::unsupported::corotype`,
  `classvariable`. These remain the real, portable work for this issue — removing a
  name from the gate's `KNOWN_UNBACKED` list as it gains a handler is the visible
  progress marker.
