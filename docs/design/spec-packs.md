# Spec packs — a Tcl DSL for command databases

> **Status:** proposal under active design, for
> [issue #1363](https://github.com/bitwisecook/tcl-lsp/issues/1363).
> Nothing here is implemented. Design-by-porting sketches live in
> [`spec-dsl-examples/`](spec-dsl-examples/) as they land.

## The problem

Users with private Tcl libraries cannot contribute their command specs to
the shipped registry, and stubs deliberately carry only a fraction of what
a `CommandSpec` can say. They need a way to author a full command database
for their own code and load it into the server — without a Rust toolchain,
and without rebuilding it for every tcl-lsp release.

## The ambition: one authoring format, two backends

The DSL is not a side door for private specs — it is the **primary
authoring format**, with two outputs:

- **Ahead-of-time**: `tcl spec build --emit rust` renders DSL sources to
  the registry `.rs` modules (the studio's `render_rs` already does
  draft → `.rs`; the DSL maps to the same draft model). The most common
  libraries — tcllib, Tk, the core packs — stay **compiled in** exactly
  as today, for zero-cost loading in the general case. Over time their
  *sources* can migrate to DSL files that generate the Rust.
- **Runtime**: private packs load the same DSL directly, paying only a
  one-time parse.

The equivalence gate falls out of that architecture: re-express a shipped
pack in the DSL, load it, and assert field-for-field equality with the
compiled spec. That round-trip is both the migration test and the proof
the DSL "entirely covers" the spec surface. The design itself is being
driven the same way — by porting hard shipped specs (`lsort`, `switch`,
TclOO definers, `upvar`) and by drafting specs for external libraries
(ticklecharts, apave, SpiceGenTcl, uncovered tcllib modules) rather than
by inventing syntax in the abstract.

## Performance: the format does not decide it

A pack — whatever its syntax — is parsed **once at load**, resolved
against the running registry's vocabularies, and inserted into the same
per-profile cached `CommandRegistry` the built-in packs live in (interned
and leaked once, keyed by content hash). After that, every query — hover,
role lookup, arity, taint — takes exactly the path a compiled-in spec
takes: same map, same struct, same field reads. Runtime cost is
**identical to compiled-in**; the only cost the format controls is a few
milliseconds of load at startup or on pack change, plus one resident copy
of the data.

The one design rule that matters for performance: packs layer into the
cached registry at workspace scope, **not** the per-document overlay path
stubs use — a pack is parsed per edit of the pack, never per edit of the
code that uses it.

## The format: a Tcl DSL, parsed by our own toolchain

We have a full Tcl compiler; the authoring format should be Tcl. A
`.tclspec.tcl` file is a declarative script in a small spec DSL:

```tcl
speclib mylib 2.1 {
    command with_var {
        synopsis {with_var varName script ?mode?}
        arity 2 3
        arg 0 -role varwrite
        arg 1 -role body
        traits {EVALUATES_CODE CREATES_SCOPE_ALIAS}
        hover -summary {Run a script with a caller variable bound.} \
              -returns {The script's result.}
    }
    command mylib::sort {
        arity 1 -1
        option -command -takes commandprefix -appends 2
        option --
        subcommand indices {arity 1 1  returns List  pure}
    }
}
```

Word spellings come from the registry's existing catalogues — traits,
roles, hook names — exactly as the Spec Studio and the reference manual
spell them, so the **?**-button help and the searchable Reference tab
document the DSL for free.

Why this beats JSON for the author:

- **The toolchain can eat its own dogfood.** `speclib`/`command` become
  registry specs themselves, with a `definition_body` grammar — the same
  machinery that gives snit and TclOO bodies highlighting, folding,
  completion, and go-to-definition with no new walker code. Authoring a
  spec pack then feels like writing tcltest or snit: full editor support,
  diagnostics for a misspelled trait or role at the point you type it.
- It is the stub language grown up — same file culture (`*.tclspec.tcl`
  beside the code), a superset of what stubs express, with a migration
  path from existing sidecars.
- Tcl quoting and line-continuation are what our users already know.

**The declaration layer is static, never a live interp.** Declarations
are read from the CST the way stubs are — loading a pack executes
nothing. If loops/templating prove necessary ("declare these 40
subcommands"), the escape hatch is evaluation inside `tcl-vm` with only
the spec-building commands exposed (the sandbox posture `tcl pkg`
already takes for untrusted build scripts), still producing pure data.

## Covering the hooks

"Entirely cover hooks" is the hard requirement, and the shipped registry
says how big it really is: of ~2,210 command modules, 44 set an
argument-role resolver, 45 a const-folder, 11 a command-prefix resolver,
and the remaining hook kinds are single digits (one `taint_sink_gate`,
one `context_gate`, one `clause_shape_check`, four literal-argument
validators, five frame effects). Hooks are the 5% tail — but they gate
the hardest commands, so the DSL takes them in four buckets:

1. **Declarative data, first.** The named-descriptor fields
   (`definition_body` grammars, `object_class`, `case_list`,
   `body_scope`, handle bindings, repeated-arg layouts, deprecation
   fixes) are plain structs — `coverage.rs` already enumerates every
   field — so they get first-class DSL forms, not code. A declarative
   clause-grammar form should also absorb most would-be
   `clause_shape_check` uses (`if`'s chain is a grammar, not an
   algorithm).
2. **Named native hooks.** Lowering / codegen / analyser hook IDs,
   shared definer grammars, and `frame_effect` descriptors are closed
   sets referenced by name. A pack reuses them; it cannot add to them.
3. **Tcl hook bodies on the VM.** Resolvers, folders, and the predicate
   gates are small **pure functions from words to data** — ideal
   sandboxed-VM material. Each hook kind gets a fixed calling
   convention (inputs: the call's words plus a context dict; output: a
   declared result protocol; error or no result = abstain), compiled to
   bytecode once at pack load, fuel-limited, no ambient authority. This
   is the one executable surface a pack has, and it is deprivileged,
   deterministic, and bounded. A const-folder even has a degenerate
   sweet case: for a command implemented in pure Tcl, "fold" can mean
   running the implementation itself on the literal arguments.
4. **Stays native.** Anything that cannot be a pure words→data function
   is a contribution, as today.

Hot-path budget: role resolvers run per call site during semantic
tokens. Built-in packs keep native pointers (bucket 2 of the
architecture above), so only pack-declared commands pay the VM cost —
and those calls are memoisable by (command, word-shape). Whether that is
enough is a measurement question, currently being benchmarked (bytecode
compile cost per hook, ns/invocation VM vs native, memo hit-rate
sensitivity); the numbers land here when they exist.

## Compatibility policy

The "ABI" is the DSL vocabulary, and it follows the tolerance rules that
let packs survive releases without rebuilds:

- Unknown property words, trait names, role names, or hook names are
  dropped with a logged notice; the rest of the spec loads. New server +
  old pack always works; old server + new pack degrades gracefully.
- A `speclib` version pragma gates hard breaks only — a word whose
  *meaning* changed, not one that was added. The server refuses a major
  it does not know and names the fix.
- The studio schema-coverage gates already force every new `CommandSpec`
  field to a named key; that key is the DSL property name, so the format
  cannot silently fall behind the registry.

## Loading and tooling

- Discovery: `tclLsp.specPacks`, plus `*.tclspec.tcl` beside a
  `tclpkg.tcl` manifest or under `.tcl-lsp/`. Name collisions with
  shipped specs are reported, shipped wins unless the pack says
  `-override`.
- `tcl spec check lib.tclspec.tcl` validates a pack from the CLI; the
  Spec Studio gains a DSL renderer beside the `.rs` and stub renderers
  (drafts round-trip through it), and the `spec-author` skill emits the
  DSL for the private-library path.
- An optional compiled cache (`tcl spec build`) is a load-time
  optimisation only — never required, never version-locked beyond the
  DSL itself.

## Phasing

1. **Freeze the surface**: one DSL word per schema key, enumerated from
   the studio's coverage gates; the syntax memo and ported examples in
   `spec-dsl-examples/`.
2. **Port the hard shipped specs** by hand (`lsort`, `switch`, `string`,
   the TclOO/snit definers, `upvar`, an iRules taint command) and let
   the syntax fight back before any loader exists.
3. **External corpus**: draft specs for ticklecharts, apave,
   SpiceGenTcl, and uncovered tcllib modules; every construct the DSL
   cannot express is a design bug filed against phase 1.
4. **Loader + equivalence gate**: static parse → draft → registry
   insert; round-trip a shipped pack and assert equality with the
   compiled specs.
5. **Hook bodies on the VM**, resolver and folder first, behind the
   measured budget; then the `--emit rust` backend, at which point
   shipped packs can migrate to DSL sources with no runtime change.

## What a pack still cannot say

Behaviour that is not a pure words→data function stays native:
commands needing new lowering/codegen/analyser specialisations are
contribution candidates, as today. Private data stays private; new
*code* in the engine goes through review.
