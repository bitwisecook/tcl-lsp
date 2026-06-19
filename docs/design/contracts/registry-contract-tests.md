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
| **LSP surface** (`test_registry_contract_e2e`) | registry lookups | `workspace/executeCommand` (`listIruleEvents`, `describeIruleEvent`, `describeIruleCommand`, `listSubcommands`) | agree with the CSVs / known ensembles |

The generated predicates were validated empirically against the live
front-end before being trusted — e.g. "command used outside its valid
events fires IRULE1001" held for 120/120 sampled commands, and the
arity predicates fired on 95/95 and 62/62 of their generated subsets.
The event-scoping valid-event set comes from the `command-info`
front-end, so that test links two front-end surfaces through the
registry.

### 2. Presence safety-net (small CSVs)

`tests/baselines/registry/*.csv` pin that **every** command (with its
arity and subcommand/switch counts), event, profile, and object is
present in the registry with basic data:

- `commands.csv` — `dialect, command, arity_min, arity_max, subcommands, switches`
- `events.csv` — `event, known, deprecated, side, multiplicity, valid_commands`
- `profiles.csv` — `profile, layer, side`
- `objects.csv` — `kind, module, object_types`

CSVs were chosen over verbose JSON dumps on purpose: a registry change is
a tiny, line-oriented, reviewable diff (one row per command), not a
multi-megabyte blob.  `test_registry_presence.py` drives the temporary
registry dumper (`scripts/registry/dump.py`) and checks its output equals
the CSVs; `test_graph_integrity.py` asserts structural invariants (stable
event order, closed profile/object reference edges) from the same dump.

Regenerate with `make gen-registry-baselines`
(`scripts/codegen/registry_baselines.py`); `--check`
(`make check-registry-baselines`, and a pytest) fails on drift.

## The registry dumper (temporary tooling)

`scripts/registry/dump.py` serialises the full structured registry as
JSON (`dump.py tcl …` / `dump.py f5 --section …`).  It is a temporary
rust-branch dev aid for bringing up the Rust front-end and for
regenerating the CSVs — **not** a shipped CLI verb or a promised surface,
and it must not be merged to `main` as one.  The full JSON is not
committed; only the compact CSVs are.

## Deterministic resolution

A few command names are overloaded across dialects (e.g. `event` is a
subcommand ensemble in core Tcl but a bare command in f5-iRules).
`CommandRegistry.get` resolves these in registration order, which varies
with load history.  The snapshot therefore resolves order-independently
via `resolve_spec` in `tooling/registry_snapshot.py` (prefer the most
dialect-specific spec), so the committed CSVs and the front-end dump
agree regardless of process history.  The behavioural generators
deliberately use the same `REGISTRY.get` path the analyser uses, so the
generated input and the front-end stay self-consistent within a run.

## Using the contract from the `rust` branch

A Rust front-end re-implements the CLI verbs (`command-info`,
`event-info`, `irule event-order`, `diag`) and the four LSP
`executeCommand` registry handlers, plus a registry dumper of its own
(the equivalent of `scripts/registry/dump.py`, kept as branch-local
tooling rather than a promised verb).  Point the e2e harness at the
native server with `TCL_LSP_SERVER_KIND=rust` and run the CLI behavioural
tests against the Rust binaries — both validate against the unchanged
CSVs and the registry-generated behavioural cases.
