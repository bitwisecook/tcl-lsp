# Registry contract & behaviour tests

This contract describes how the command registry and the iRules
event / profile / object graphs are tested **through the front-ends**,
with the registry acting as both the *generator of inputs* and the
*oracle for expected outputs*.  The aim is maximum behavioural value and
a language-agnostic contract a future Rust front-end (the `rust` branch)
passes unchanged: the contract is the front-end's observable behaviour,
not any Python API.

Everything lives under `tests/registry_contract/` (CLI-driven) and
`tests/lsp_e2e/test_registry_contract_e2e.py` (LSP-driven).

## Two layers

### 1. Behavioural tests (the bulk of the value)

The registry generates real Tcl scripts and iRules, feeds them to the
front-ends, and asserts the actual analysis.  Generators live in
`tests/registry_contract/_generators.py`; each yields a `DiagCase`
(source + the diagnostic codes it must / must not raise), checked via the
real `tcl diag` / `f5 irule` front-ends.

| Category | Generated input | Front-end | Oracle |
|---|---|---|---|
| **Arity** (`test_arity_behaviour`) | a call of every plain-positional command with too few / too many / exactly-min args | `tcl diag --json` | E002 / E003 fire; a valid call is clean |
| **Subcommands** (`test_subcommand_behaviour`) | every ensemble called bare / with a bogus sub / with a real sub | `tcl diag --json` | E001 / W001 fire; a real sub is clean |
| **Event scoping** (`test_irule_event_scoping`) | `when EVENT { command }` for every iRules command | `tcl diag --dialect f5-irules` | IRULE1001 fires iff the command is used outside its valid events; any-event commands never warn; unknown event → IRULE1002 |
| **Event graph** (`test_irule_event_graph`) | an iRule with many `when` blocks in scrambled order | `f5 irule event-order` | returned in the registry's canonical firing order |

The generated predicates were validated empirically against the live
front-end before being trusted — e.g. "command used outside its valid
events fires IRULE1001" held for 120/120 sampled commands, and the
arity predicates fired on 95/95 and 62/62 of their generated subsets.
The event-scoping valid-event set comes from the `command-info`
front-end, so that test links two front-end surfaces through the
registry.

### 2. Presence safety-net (retired)

The Python era carried a set of golden CSVs
(`tests/baselines/registry/*.csv`, later ported to
`rust/tcl-lsp-server/tests/fixtures/registry/*.csv`) that pinned every
command (with its arity and subcommand / switch counts), event, profile,
and object present in the registry.  Those CSVs and their comparison test
have been **retired** — they compared the live registry against a
Python-derived dump, which is no longer meaningful now that Python has been
removed and the behavioural sweep tests above are the contract.  Presence
is instead asserted directly against the in-process registry by
`rust/tcl-registry/tests/registry_commands.rs` and
`rust/tcl-registry/tests/registry_sweep.rs`.

## Deterministic resolution

A few command names are overloaded across dialects (e.g. `event` is a
subcommand ensemble in core Tcl but a bare command in f5-iRules).
`CommandRegistry.get` resolves these in registration order, which varies
with load history.  The row builders therefore resolve order-independently
via `resolve_spec` in `tooling/registry_snapshot.py` (prefer the most
dialect-specific spec), so the committed CSVs are stable regardless of
process history.  The behavioural generators deliberately use the same
`REGISTRY.get` path the analyser uses, so the generated input and the
front-end stay self-consistent within a run.

## How the contract runs today

The native front-ends expose the CLI verbs (`command-info`, `event-info`,
`irule event-order`, `diag`) and the LSP `executeCommand` registry
handlers.  The behavioural sweep and presence checks read the in-process
registry directly (no JSON wire, no golden dump to reproduce) in
`rust/tcl-registry/tests/registry_sweep.rs` and
`rust/tcl-registry/tests/registry_commands.rs`.  The LSP-driven surface is
exercised by the native e2e suite under `rust/tcl-lsp-server/tests/`.
