# Cross-file command resolution — proposal (not yet implemented)

**Status:** design proposal, scoping issue #923's regression report. No code
here has landed. Follows the same "two-halves, sound-by-abstention" pattern
as the (also unshipped) [TclOO class-binding lattice
experiment](tcloo-mro-lattice.md) — read that first if you haven't; this
proposal leans on it directly for phase 3.

## Problem

[command-resolution.md](contracts/command-resolution.md) establishes one
algorithm, one Rust home (`tcl_syntax::naming`), and a conformance-vector
gate that every consumer must pass. Every consumer **except one** goes
through it: `WorkspaceIndex::invocations_of` / `proc_definitions` /
`class_definitions` (`tcl-lsp-core/src/workspace_index.rs`) — the cross-file
half of Find References / Rename / Call Hierarchy — reimplements its own
ad hoc matching (literal-text equality, a bareword fallback gated on
workspace-wide name uniqueness). It predates the contract and was never
folded in.

PR #924 changed what `resolved_qualified_name` means (correct, required for
issue #923's single-file fix) without touching that one remaining
non-conformant consumer. Confirmed fallout, precisely isolated (see the
issue thread for the full writeup and repro steps):

1. A cross-file call that needs to fall through past the caller's own
   namespace (concretely, via `namespace path`) to reach its target now
   goes unfound, whenever the target's simple name isn't unique
   workspace-wide (the workspace-uniqueness bareword fallback is what used
   to mask this).
2. `proc_definitions`/`class_definitions` match candidate declarations by
   simple name only, so an unrelated same-named proc/class in a different
   namespace is surfaced as if it were the same symbol — a false positive,
   independent of #1.
3. TclOO methods have **no** cross-file path at all —
   `resolve_workspace_symbol` (`tcl-lsp-server/src/lib.rs:3631-3675`) only
   ever resolves proc/class *names*, never method names. Given "one file
   per class" is a common Tcl layout, this is likely the largest
   contributor to the reported "no references in other files" experience,
   and it needs no special trigger condition.
4. Separately (pre-existing, not caused by #924): clicking directly on a
   relative-qualified call site finds nothing, cross-file *or* same-file —
   `resolve_workspace_symbol`'s word lookup does plain string equality
   instead of consulting the invocation's already-resolved candidate.
5. Command names held in variables (`set cmd helper; $cmd`) are a
   documented, accepted limitation today, not a regression — but it's the
   same shape of problem as #1, and the same fix generalises to it.

## Why "patch `invocations_of` with more clauses" is the wrong move

That function already has four special-cased match clauses, added
incrementally, and #924 just demonstrated the failure mode of that
approach: every clause is a snapshot of "what the analyser happened to
produce" at the time it was written, and the analyser's output is not a
stable contract that function ever agreed to track. A fifth clause fixes
today's repro and leaves the same class of bug for the next analyser
change. The actual bug is architectural: **there are two independent
implementations of "does this call resolve to that definition," and only
one of them is conformance-gated.**

## Proposed architecture: one oracle, workspace-scoped

`resolve_command_with` is already shaped for this — its `exists` parameter
is a generic `FnMut(&str) -> bool`, not hardcoded to any particular data
source:

```rust
pub fn resolve_command_with<S: AsRef<str>, F: FnMut(&str) -> bool>(
    namespace: &str,
    path: &[S],
    cmd_name: &str,
    mut exists: F,
) -> Option<String>
```

Every existing caller closes over one file's `all_procs` / `all_classes` /
`command_aliases` / `renamed_commands` + registry builtins (see
`finalise_invocation_resolutions`, `scope.rs:361-368`). The fix is to give
`WorkspaceIndex` an `exists`-shaped oracle over the **merged** workspace —
every file's procs/classes/aliases/renames, keyed the same way — and run
the *identical* candidate list through it instead of `invocations_of`'s
bespoke clauses:

```rust
impl WorkspaceIndex {
    /// Workspace-wide existence oracle for `resolve_command_with`: true when
    /// `qualified` names a real proc, class, alias, or rename target
    /// anywhere in the indexed workspace (any file, including `exclude_uri`
    /// itself — a call can legitimately resolve to a definition in its own
    /// file that just wasn't "known" from a pathless walk-time guess).
    fn workspace_command_exists(&self, qualified: &str) -> bool { ... }
}
```

`invocations_of` becomes: take the call's recorded candidate list (see
below — this requires the analyser to *keep* the list, not collapse it to
one guess), run `resolve_command_with` against
`workspace_command_exists`, and match if the winner equals the target.
Clauses 1/2/4 (literal-text equality, bareword-fallback-gated-on-uniqueness)
retire entirely — they were a weaker approximation of exactly this. This
directly fixes #1 (`namespace path` fallthrough now finds the workspace
definition, not just the current file's) with no ambiguity gating needed,
because the oracle itself is precise instead of a uniqueness proxy.

**This requires one analyser change**: `finalise_invocation_resolutions`
currently collapses to a single `Option<String>` (`resolved_qualified_name`)
and *keeps the local-first guess* when nothing is known locally (`scope.rs`
lines 394–400) — that guess is exactly the thing #1 breaks on. Recording
the **full candidate list** (`Vec<String>`, in priority order — cheap,
`command_resolution_candidates` already computes it) alongside the existing
single-value field lets the workspace-scoped resolver try all of them
against its wider oracle, rather than trying to reverse-engineer candidates
from one already-collapsed string. `resolved_qualified_name` stays as-is
for every same-file consumer (no behaviour change there); the candidate
list is additive.

## Fixing the false positive (#2)

`class_lattice.rs`'s `NsContext` already states the right discipline for
this exact shape of problem: "never matched to a same-tailed class in an
unrelated namespace, so cross-file resolution cannot manufacture a
confident false resolution from a namespace collision." `proc_definitions`
/ `class_definitions`'s "declarations to include" branch needs the same
rule — require the *resolved* qualified name (or an exact qualified match),
never a bare simple-name scan, when deciding which declarations are "the
same symbol" for the include-declaration / rename-target set.

## Fixing the call-site asymmetry (#4)

`resolve_workspace_symbol` re-derives "what symbol is under the cursor"
from raw word text. When the cursor sits on a call site (not a
declaration), that information already exists, correctly resolved, in
`analysis.command_invocations[i].resolved_qualified_name` (and, post
phase 1, the candidate list) — look it up there first and only fall back
to word-text matching when the position isn't a recorded invocation at
all (e.g. cursor on a bare word that isn't a call head). No new resolution
logic, just consuming data the walk already produced instead of recomputing
it worse.

## Phase 3 — TclOO cross-file (#3)

`class_lattice.rs` already designed the cross-file half of this problem for
*object→class binding* and named it explicitly as ablation **A3**:
"resolve against a corpus-merged class index instead of the per-file one,"
with `cross-file-miss` as one of the ⊤-abstention reasons. That module is
unshipped, but its design is directly reusable for **method** resolution,
which is the actual gap here:

- Extend `WorkspaceIndex` with a method table per indexed class (it already
  has `defined_methods: Vec<String>` on `WorkspaceClass` — the class side
  is half-done).
- Extend `resolve_workspace_symbol` to check method names against it (mirroring
  the existing proc/class branches), and `cross_document_references` to
  gather `$obj method` / `my method` call sites the same way
  `invocations_of` gathers proc calls.
- Reuse `class_lattice.rs`'s `NsContext` for the class-name half of `$obj
  method` resolution (which namespace does `Foo` refer to here?) rather
  than hand-rolling a second version of it — this also means shipping
  method-level cross-file references is a real, low-risk motivation to
  finally wire up the class lattice experiment, gated the same
  sound-by-abstention way it already is internally.

## Phase 4 — command names in variables (#5, the "tricky indirection" case)

This is the one genuinely new piece, and the most speculative — flagging
the open questions rather than a firm design.

The SCCP lattice (`sccp.rs`, `analyses.rs`) already computes exactly the
right fact — `LatticeValue::Const(ConstValue::String(s))` for a
provably-constant SSA value, `ConstSet(Vec<ConstValue>)` for a small
join of possibilities (e.g. an `if/else` assigning two different literal
command names), `Overdefined`/`Unknown` otherwise. `set cmd helper; $cmd`
is precisely "ask SCCP what `cmd`'s SSA value is at the call site": a
`Const` resolves exactly like a literal bareword call to that string
through the same `resolve_command_with` path above; a `ConstSet` resolves
each member and unions the reference sets (a real ambiguous dispatch has
multiple *possible* runtime targets — Find References should surface all
of them, matching the `Set` widening `class_lattice.rs` already does for
object types rather than collapsing to ⊤); `Overdefined`/`Unknown`
abstains, same as today's documented limitation — never a wrong guess.

**Open question, not yet resolved**: the analyser's walk (which records
`command_invocations`) and the SSA/CFG/SCCP pipeline
(`compilation_unit.rs`, `build_ssa` + `sccp_with_extra_escaping`) are
currently two separate passes — `analyser/*.rs` does not import `ssa`,
`cfg`, or `sccp` at all. Whether SSA/SCCP is already computed for every
open document as a byproduct of the optimiser/diagnostics pipeline (in
which case this is "expose an already-paid-for result to one more
consumer") or would need to be built freshly for this purpose (a real new
per-document cost, needing its own scoping/benchmarking) needs to be
established from the LSP request pipeline before committing to this
phase's cost. Suggest resolving that question first, as a short spike,
before scoping the rest of phase 4 in detail.

## Phase 5 — resolving into library/package files (`TCL_LIBRARY`, `TCLLIBPATH`, editor config)

Corrected from an earlier draft of this doc, which mischaracterised this as
"not implemented at all." That's wrong in the way that matters most: the
**discovery and configuration layer is already fully built** and is more
thorough than most of what's proposed above needed to be —

- `tcl_install::discover` finds installed Tcl trees by walking
  platform-specific search bases plus `$TCL_LIBRARY` (directly) and
  `$TCLLIBPATH` (`tcl_install.rs:54-106`), looking for a directory
  containing `init.tcl`, with best-effort version detection per
  installation.
- `effective_auto_path` (`tcl-lsp-server/src/lib.rs:10784-10791`) layers,
  in priority order: the editor's `tclLsp.libraryPaths` setting (a real,
  documented VS Code config key, `editors/vscode/package.json:3037-3046`)
  → user `config.ini` `[global] libraryPaths` → per-workspace
  `.tcl-lsp.ini` `[project] libraryPaths` → discovered installations'
  `auto_path` → `$TCLLIBPATH`.
- `PackageResolver` (`package_resolver.rs`) then mirrors C Tcl's actual
  loading machinery over that path set — structurally parsing
  `pkgIndex.tcl` (`package ifneeded` → source files, matching
  `tclPkgUnknown`) and `tclIndex` (`auto_index` proc→file, matching
  `auto_load_index`/`auto_qualify`) with the real lexer, differentially
  tested against actual `tclsh`. It already knows, for any `package
  require`d or auto-loadable name, exactly which file on disk defines it.

**The actual gap**: `scan_workspace_folders` builds this `PackageResolver`
from the full effective `auto_path` (so it's completely config/env-aware),
but only ever calls `workspace_index.add_document` for files under the
*workspace folders* (`collect_tcl_files(root, ...)` over `roots`, i.e. the
editor's open folders) — `PackageResolver`'s resolved `source_files`
(`PackageInfo`) and `source_file` (`AutoIndexEntry`) never get analysed
into `WorkspaceIndex` at all. Today `PackageResolver`'s only consumer is
the W120 (`refine_w120_diagnostics`) existence check — "is this package
resolvable" — not the actual proc/class content of the resolved files.
Definition/References/Rename/Call Hierarchy accordingly only ever see
workspace files, never library ones, entirely independent of whether the
library was correctly located.

Proposed fix, once phase 1's workspace-scoped `exists` oracle exists: give
it a **second-tier source** — when a candidate isn't found in
`WorkspaceIndex`'s already-analysed documents, consult `PackageResolver`
for a file that would define it (`AutoIndexEntry`/`PackageInfo` lookup by
qualified name), lazily analyse *that* file on demand (not eagerly — a
workspace can trivially pull in all of Tk + tcllib, and nobody wants every
open project to eagerly parse the whole standard library on startup),
memoise the result the same way `analysis_for` memoises workspace
documents, and merge it into the oracle. This makes library resolution
"just" a lazily-populated extra layer behind the same oracle interface —
no separate mechanism, and it inherits phase 1's soundness properties
(never guesses; abstains when the file can't be parsed or the name isn't
found in it).

Scope note: dialect matters here too — a BIG-IP/iRules project's "library"
is a different concept (F5-provided commands, not `pkgIndex.tcl`), already
modelled separately by `tcl-registry`'s dialect-specific command tables;
this phase is about *Tcl-proc-level* libraries (tcllib, Tk, project-private
packages), not re-deriving iRules' registry-driven command set.

## Suggested delivery order

1. Workspace-scoped `exists` oracle + candidate-list recording — fixes #1,
   #2. Smallest, highest-value, no new lattice work, purely wiring the
   already-canonical algorithm one layer further out.
2. Call-site lookup via `command_invocations` instead of word text — fixes
   #4. Independent of phase 1, can land in either order.
3. TclOO method cross-file, reusing `class_lattice.rs`'s design — fixes
   #3, and is a real justification to ship (part of) that experiment.
4. SCCP-backed command-name-in-variable resolution — the "tricky
   indirection" case, pending the spike above.
5. Library/package resolution as a lazy second tier behind the same
   oracle — the discovery/config half needs no new work, only the
   lazy-analyse-and-merge wiring described above.

## Testing

Per the contract's own discipline ("adding a resolution behaviour = adding
a vector"): the existing `command_resolution_conformance.rs` family
(analyser, VM, WASM runtime, tcl-syntax-vs-tclsh) is all single
compilation-unit. This needs a **multi-file** vector format — same
`(namespace, path, name) → candidate` shape, but seeded across N synthetic
documents with the target definition in a different document than the
call — so cross-file resolution is pinned the same non-negotiable way
same-file resolution already is, and a future analyser change that breaks
this can't ship silently the way #924 did.
