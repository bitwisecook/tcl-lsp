# Registry contract & behaviour tests

This contract describes how the command registry and the iRules
event / profile / object graphs are tested **through the front-ends**,
with the registry acting as both the *generator of inputs* and the *oracle for
expected outputs*. The contract is the front-end's **observable behaviour**,
not any internal API.

## Behavioural sweeps

The registry generates real Tcl scripts and iRules, feeds them to the
front-ends, and asserts the actual analysis. Each generated case pairs a
source with the diagnostic codes it must and must not raise.

| Category | Generated input | Oracle |
|---|---|---|
| **Arity** | a call of every plain-positional command with too few / too many / exactly-min arguments | E002 / E003 fire; a valid call is clean |
| **Subcommands** | every ensemble called bare, with a bogus subcommand, and with a real one | E001 / W001 fire; a real subcommand is clean |
| **Event scoping** | `when EVENT { command }` for every iRules command | IRULE1001 fires iff the command is used outside its valid events; any-event commands never warn; an unknown event is IRULE1002 |
| **Event graph** | an iRule with many `when` blocks in scrambled order | returned in the registry's canonical firing order |

Each generated predicate is only trustworthy if it was validated against the
live front-end before being relied on — a generator that produces cases the
front-end never flags proves nothing. The event-scoping valid-event set comes
from the `command-info` surface, so that sweep links two front-end surfaces
through the registry.

## Presence

Presence is asserted **directly against the in-process registry** — no JSON
wire, no golden dump to regenerate — in
`rust/tcl-registry/tests/registry_commands.rs` and
`rust/tcl-registry/tests/registry_sweep.rs`.

A golden-CSV safety net is deliberately *not* used. Comparing the live
registry against a committed dump only ever restates what the dump was
generated from, and every registry edit turns into a two-file change with no
added signal.

## Deterministic resolution

A few command names are overloaded across dialects — `event` is a subcommand
ensemble in core Tcl but a bare command in f5-iRules. Resolution must not
depend on registration order, which varies with load history: the most
dialect-specific spec wins. The behavioural generators deliberately use the
same lookup path the analyser uses, so the generated input and the front-end
stay self-consistent within a run.

## Front-end surfaces

The native front-ends expose the CLI verbs (`command-info`, `event-info`,
`irule event-order`, `diag`) and the LSP `executeCommand` registry handlers.
The LSP-driven surface is exercised by the e2e suite under
`rust/tcl-lsp-server/tests/`.
