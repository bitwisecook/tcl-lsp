# Registry contract & behaviour tests

This contract describes how the command registry and the iRules
event / profile / object graphs are tested, with the registry acting as
both the *generator of inputs* and the *oracle for expected outputs*.
The contract is the front-end's observable behaviour, not any host API —
which is what let the Python-era suite be retired without losing the
contract itself.

## What runs today

The behavioural sweep and presence checks read the in-process registry
directly in `rust/tcl-registry/tests/registry_sweep.rs` and
`rust/tcl-registry/tests/registry_commands.rs`:

- **Accessor sweep** (`sweep_every_command_every_accessor`) — every
  command in every dialect is run through every registry accessor, with
  arity self-consistency (`assert_arity_consistent`), dialect-set nesting
  between a command, its forms, and its subcommands, and trait-membership
  self-consistency asserted per spec.
- **Contract tests** (`registry_commands.rs`) — the consumer-visible
  keyword sets (dispatch keywords, method-context commands, and friends)
  are asserted equal to the trait-carrying specs, per dialect, so a spec
  edit cannot silently change a consumer surface.
- **Front-end surfaces** — the CLI verbs (`command-info`, `diag`,
  `registry-dump`) and the LSP `executeCommand` registry handlers are
  exercised by the native end-to-end suites under
  `rust/tcl-lsp-server/tests/`.

## The retired Python layers (history)

The Python era drove the same contract from the outside: generators under
`tests/registry_contract/` produced a call of every command with too few /
too many / valid arguments, every ensemble with bogus and real
subcommands, and `when EVENT { command }` for every iRules command, then
asserted the diagnostic codes through the real `tcl diag` / `f5 irule`
front-ends. The generated predicates were validated empirically before
being trusted (e.g. the event-scoping predicate held for 120/120 sampled
commands).

A presence safety-net of golden CSVs pinned every command, event,
profile, and object. Both layers compared the live registry against a
Python-derived oracle, so they were retired with Python; presence is now
asserted directly against the in-process registry by the sweeps above.

## Deterministic resolution

A few command names are overloaded across dialects (e.g. `link` in core
Tcl 9 versus tcllib's `ooutil`). `CommandRegistry::get` returns the
last-registered spec, so order-sensitive consumers resolve through
`DialectProfile::resolve_command` (the most dialect-specific visible
spec), and whole-grammar generators ask over `CommandRegistry::specs`
instead of any single-spec lookup. The sweeps deliberately use the same
resolution path the analyser uses, so the tested surface and the shipped
behaviour stay self-consistent within a run.
