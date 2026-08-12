# Spec packs — a Tcl DSL for private command databases

> **Status:** proposal, for discussion on
> [issue #1363](https://github.com/bitwisecook/tcl-lsp/issues/1363).
> Nothing here is implemented.

## The problem

Users with private Tcl libraries cannot contribute their command specs to
the shipped registry, and stubs deliberately carry only a fraction of what
a `CommandSpec` can say. They need a way to author a full command database
for their own code and load it into the server — without a Rust toolchain,
and without rebuilding it for every tcl-lsp release.

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

**Evaluation is static, never a live interp.** The pack is parsed with
the same analyser that parses stubs — declarations are read from the
CST, not executed, so a pack cannot run anything. If loops/templating
prove necessary ("declare these 40 subcommands"), the escape hatch is
evaluation inside `tcl-vm` with only the spec-building commands exposed
(the sandbox posture `tcl pkg` already takes for untrusted build
scripts), still producing pure data. Start static; add the VM path only
on demand.

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

## What a pack cannot say

Function-pointer behaviour (bespoke resolvers, folders, gates) stays
out: hooks are referenced by name from the closed tables, and commands
that need new code are contribution candidates. That is the boundary we
want — private data stays private, behaviour that needs code goes
through review.
