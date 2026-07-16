# Name resolution centralization — audit + proposal (not yet implemented)

**Status:** design proposal. No code here has landed. Written after a
four-way parallel audit (command-name target selection, variable
resolution, class resolution, and the VM/WASM runtime/codegen picture)
triggered by scoping the fix for issue #923. Companion to
[cross-file-command-resolution-lattice.md](cross-file-command-resolution-lattice.md),
which covers the workspace-wide reference-*enumeration* half of the
problem (given a known target, find its call sites across files) in
depth — this document covers everything else the audit found, and is
broader than #923's scope.

## Executive summary, in order of what needs attention first

1. **Target selection is independently reimplemented in ~17 places**
   (Part A). The canonical, correct implementation already exists
   (`definition.rs::resolve_called_proc`) — most call sites just don't
   use it. Several of the duplicates are **live, silent-correctness bugs**,
   not missing features: Rename and Call Hierarchy can act on the *wrong*
   same-named symbol when triggered from a call site; Linked Editing Range
   can live-link an unrelated call site during rename-as-you-type. This is
   the most urgent finding in this document and is mechanically the
   cheapest to fix.
2. **Cross-file reference enumeration** (`WorkspaceIndex`) — the
   already-scoped #923 regression. See the companion doc.
3. **Variable resolution has four independent implementations** (VM, WASM
   runtime, static analyser, compiler SSA/place) with no shared algorithm
   and no conformance suite, unlike commands (Part C). Bigger, more
   structural lift than commands were — genuinely different data models,
   not just duplicated logic — recommend a scoping spike rather than
   reflexively copying the command-resolution pattern.
4. **Class resolution is forked three ways at the static/analyser level**
   (Part D), and one of the three implementations was verified against
   the VM's actual runtime behavior to be **wrong** — it implements an
   ancestor-namespace walk that real Tcl's `superclass`/`mixin` resolution
   doesn't do.
5. **Runtime/codegen backends are in better shape than the rest** (Part
   E) — confirmed genuinely integrated with the shared algorithm, plus
   one factual correction to the existing contract doc and a performance
   note worth heeding before centralizing further.

## Part A: target selection — "which definition does this cursor refer to"

Command resolution's contract (`command-resolution.md`) covers **half**
of what "resolve a name" means for an LSP: given a known target
definition, enumerate every call site that refers to it
(`invocation_references_named` in `references.rs`, genuinely unified by
d73463b). The *other* half — given the cursor sitting on a call site, decide
*which* definition it is — was never brought under the same discipline.
`definition.rs` already has the right answer to this (`resolve_called_proc`
→ `proc_visible_from_namespace` → `tcl_syntax::naming::bareword_resolution_candidates`,
namespace-aware, used correctly by `hover.rs`'s proc path). Nearly
everything else re-derives its own, worse version: a raw
`analysis.all_procs.iter().find(|(qn, p)| p.name == word || …)` scan with
no namespace awareness and — since it iterates a `HashMap` — an
unseeded/randomized tie-break among same-named candidates that can differ
across server restarts.

### Tier 1 — confirmed live, silent-correctness bugs

| Where | What breaks |
|---|---|
| `rename.rs:663-676` (`rename_proc`), `:744-752` (`rename_class`) | Renaming from a bareword **call site** (the ordinary way to trigger a rename) can resolve to a *different* same-tailed proc/class elsewhere in the workspace, rewriting its declaration and call sites while leaving the one you actually clicked untouched. |
| `call_hierarchy.rs:85-99`, `:127-145` | Same shape — incoming/outgoing call-hierarchy views seeded from the wrong node. The function's own doc comment states the ambiguity risk, then resolves it with the same non-namespace-aware scan anyway. |
| `references.rs:340-359` (`class_references`), `:388-407` (`proc_references`) | Same shape when Find-All-References / Document-Highlight is invoked from a call site rather than the declaration — arguably the more common trigger. |
| `tcl-lsp-server/src/lib.rs:3631-3675` (`resolve_workspace_symbol`) | A **fourth** copy of the pattern, gating *cross-document* rename and references — the wrong-target risk now potentially propagates an edit into a different file entirely. |
| `linked_editing_range.rs:90-100`/`:148-150` | A distinct shape: `if !naive_match && !resolved_match { continue }` — an **OR**, so a call site the naive check pulls in wrongly stays linked even when the canonical `resolved_qualified_name` says it belongs to someone else. Concrete repro: `proc ::a::greet {} { namespace eval ::b { greet } }` with a separate `::b::greet` defined — the inner bareword genuinely dispatches to `::b::greet` at runtime, but textually equals `::a::greet` and sits lexically inside its body, so linked-editing links them. Renaming `::a::greet` live would mutate that unrelated call site too. Not touched by #924 — pre-existing. |
| `class_hierarchy.rs:258-308` (`resolve_class_name`) | **Confirmed wrong**, not just duplicated — verified against the VM's actual `superclass`/`mixin` resolver (`tcl-vm/src/cmd_oo.rs:193-211`, the real ground truth), which implements Tcl's actual one-hop rule (absolute as-is, else current namespace only, else global — no ancestor walk). The analyser's version instead walks *every* enclosing namespace level up to global. A bare `superclass Base` written inside `::a::b::Sub` where `Base` only exists at `::a::Base` is genuinely broken in real Tcl (`superclass` errors at class-definition time) — but the analyser confidently links it anyway, feeding a wrong MRO into method resolution, type hierarchy, and the W308 diagnostic for code that doesn't actually run. |

### Tier 2 — real, narrower blast radius

- `implementation.rs` (`class_matches`/`strip_colons`): `strip_colons` only
  trims *leading* colons rather than extracting a tail, so Go-to-Implementation
  fails to find a subclass that references a namespaced base class *bareword*
  — confirmed concretely, and it's the idiomatic way to write it. The file's
  own test suite only exercises global-namespace classes, so this wasn't
  caught locally.
- `signature_help.rs` and `inlay_hints.rs`: byte-for-byte identical bespoke
  matcher in both files, with **no** position/declaration-tier gate at all
  (unlike rename/references/call-hierarchy, there isn't even a "cursor is
  exactly on the decl" precise path first).
- `hover.rs`'s `lookup_class`/`alias_hover_text` and `type_hierarchy.rs`'s
  `prepare`: same ungated shape — wrong class's docs or hierarchy root can
  surface. (Proc hover in the same file is fine; it already goes through
  `resolve_called_proc`.)
- `workspace_index.rs`'s `proc_definitions`/`class_definitions`: worse than
  `invocations_of` even — completely ungated, no `bare_is_safe`-style check
  at all. Also a correction to the mechanism I'd attributed to the #923
  regression: `scan_workspace_folders` runs the *full* `Analyser::analyse`
  even for background-scanned/unopened files, so `resolved_qualified_name`
  is populated there too — the root issue is specifically that
  `invocations_of`'s literal-text/bareword clauses never consult it at all,
  not that background scans lack the data. Doesn't change the confirmed
  repro from the companion doc, just sharpens why.

### Tier 3 — lower confidence, narrow trigger, or currently dead

`minify.rs` (real, narrow — wrong parameter list attached during
minification on a name collision); `graphs.rs` (correctly *prefers*
`resolved_qualified_name` already — its naive fallback is provably
unreachable with today's callers, but fragile if that changes);
`type_definition.rs` (narrow, gated behind an exact key lookup first);
`tcl-mcp/src/tools.rs`'s `generate_docstring` (bespoke fallback, claims to
"mirror `AnalysisResult.find_proc`" — no such method exists anywhere in the
workspace, so that comment is stale); `class_lattice.rs`'s
`resolve_class_name` (a *second* independent ancestor-walk implementation,
same anti-pattern as the Tier-1 `class_hierarchy.rs` one, but the module is
an explicitly unshipped experiment); `mro_interproc.rs` (a `cargo run
--example` research harness, not shipping code).

### Checked, and confirmed NOT findings

`tcl-registry`'s ensemble/subcommand resolvers (a different question —
literal/prefix matching against a static spec, no user namespaces
involved); `rename.rs`'s `is_builtin_command_name` (bare-name comparison is
*correct* here since builtins are always global); `tcl-vm/src/cmd_oo.rs`'s
`resolve_class` (hand-rolled rather than calling the shared helper, but
verified behaviorally equivalent — a style nit, not a bug); `code_lens.rs`,
`declaration.rs` (both already route through the canonical paths);
`tcl-explorer`, `tcl-cli`, `tcl-debugger` (no reimplementation — different
domains, or consuming already-resolved VM output).

### Proposed fix

Route every Tier 1/2 site through `definition.rs::resolve_called_proc` (or
a class-oriented analogue built the same way — `proc_visible_from_namespace`
generalizes cleanly) instead of each re-deriving its own
`.iter().find(name-equality)` scan. This is mechanically simple — collapsing
~15 duplicate implementations onto 1–2 existing, already-correct ones — and
should land **before** anything in the companion doc's phase list: it fixes
live silent-corruption bugs, needs no new lattice/oracle work, and phase 2
of the companion doc (call-site lookup for cross-document resolution) turns
out to be largely subsumed by this fix once `resolve_workspace_symbol` is
brought in line with it too.

## Part B: cross-file reference enumeration

Covered in full in the companion doc. Recap for context: `WorkspaceIndex`
reimplements its own literal-text/bareword matching instead of using
`resolve_command_with`, which #924 silently broke for calls needing
multi-candidate fallthrough (`namespace path`) combined with a
workspace-wide simple-name collision. Same family of problem as Part A
(a consumer that never got folded into the shared algorithm), different
half of the problem (enumeration, not selection), different fix (a
workspace-scoped `exists` oracle, not a helper-function swap).

## Part C: variable resolution — four independent implementations, no contract

Unlike commands, variable resolution has **no** single-algorithm-plus-
conformance-suite treatment, and the documentation meant to cover it is
misleading: `command-resolution.md` points to `namespace-model.md` for
variables, but that doc is actually about a different "namespace" concept
(F5 iRules protocol/command-prefix namespaces) and doesn't describe a
variable-resolution algorithm at all. `runtime-variable-frame-model.md`
looks more relevant but explicitly self-describes as an aspirational
"if starting over" design, not a description of the current
implementation, and never mentions `tcl-vm`, `tcl-compiler`, or
`tcl-lsp-core`. No conformance-vector suite analogous to
`command_resolution_vectors.txt` exists for variables.

Four separately-implemented resolution walks:

- **Runtime**: `runtime/rust/src/namespace.rs`'s `var_home` — an
  arena/`NsId`-tree walk.
- **Bytecode VM**: `tcl-vm/src/interp.rs`'s `locate_from`/`locate` —
  structurally different: a qualified name is a flat string keyed into
  frame 0's `locals` map, not walked through a namespace-child arena. No
  `var_home`-equivalent function exists here.
- **Static analyser** (feeds LSP hover/definition/references/rename/
  completion): built by `analyser/handlers.rs`'s
  `handle_global_command`/`handle_variable_command`/`handle_upvar_command`/
  `handle_namespace_upvar_command`, walked by `analyser/scope.rs`. Consumed
  through one shared chain-walker,
  `tcl-lsp-core/src/definition.rs::lookup_var_in_scope_chain` — good
  centralization *within the LSP crate*, but over a scope tree nothing
  else shares.
- **Compiler SSA/place** (memory-SSA alias/overlap analysis):
  `tcl-compiler/src/var_resolve.rs`'s `ResolveContext`, populated via
  `place_bridge.rs`.

There's also a partially-adopted low-level helper,
`tcl-compiler/src/var_scoping.rs` (declaration-index classification),
reused by `place_bridge.rs` and `var_escape/*` — but **not** by
`analyser/handlers.rs`, which reimplements the same grammar inline and
misses one of the helper's exclusion rules in the process.

**This is a bigger lift than command resolution was**, and shouldn't be
approached by reflexively copying that pattern: the VM's flat-map-per-frame
model and the runtime's arena-tree model are genuinely different data
structures for genuinely different execution strategies, not just two
implementations of the same lookup. Recommend treating this as its own
scoping spike rather than folding it into this proposal's delivery order.
Lower-risk, still-valuable near-term steps that don't require resolving the
bigger question: fix the two broken/misleading doc pointers, and stop
`analyser/handlers.rs` from re-deriving `var_scoping.rs`'s grammar inline.

## Part D: class resolution — centralized at runtime, forked three ways statically

Good news first: at the two **execution** backends, class resolution is
already correctly centralized, because TclOO classes are just ordinary
command-table entries. The VM's `Command::Object` is resolved through
`lookup_command` → `resolve_command_fqn`, which calls
`tcl_syntax::naming::resolve_command_with` directly — no special-casing.
The WASM runtime's `OoObject` is resolved through the same `home_of`
structural mirror everything else uses. This confirms the hypothesis that
"classes are commands" holds all the way down at the execution layer.

The **static/analyser** side forked into three independent, mutually
disagreeing implementations, none calling `tcl_syntax::naming`:

1. `class_hierarchy.rs::resolve_class_name` — exact match → **ancestor-
   namespace walk** → globally-unique-tail fallback. Feeds the MRO builder,
   `type_hierarchy.rs`, and `workspace_index.rs`'s cross-file class/method
   lookups. **Confirmed wrong** against the VM's actual behavior (see Part
   A, Tier 1) — real Tcl's `superclass`/`mixin` resolution is a one-hop
   rule (current namespace, then global), not an ancestor walk.
2. `class_lattice.rs::resolve_class_name` (+ `NsContext`) — exact →
   offset-based enclosing-namespace walk → explicit tracked `namespace
   import` prefixes → global. Its own doc comment states it exists
   specifically to fix flaws in a "unique-tail heuristic" — i.e. the code
   itself documents disagreeing with implementation #1. Experimental,
   unwired into any shipping diagnostic.
3. `analyser/diagnostics/var_command.rs::canonicalise_class_name` — a
   third, simpler heuristic (exact, else global-prefix, else give up),
   feeding the W308 diagnostic's constructor-object-type harvesting.

`tcloo-implementation.md` is also stale here — it describes the pre-Rust-port
Python modules and says nothing about this three-way split.

**Proposed fix**: collapse onto one implementation that matches the VM's
verified-correct one-hop rule — ideally by calling `tcl_syntax::naming`
directly (ordinary namespace-relative resolution, since a class name IS a
command name) rather than any of the three hand-rolled walks. This directly
fixes the Tier-1 wrong-MRO bug and removes two further duplicate
implementations in the same motion.

## Part E: runtime and codegen — the encouraging part, with corrections

The two execution backends are genuinely well-integrated with the shared
algorithm, confirmed by reading the actual call sites rather than trusting
the contract doc's summary:

- **VM** (`tcl-vm/src/interp.rs`): `lookup_command` calls
  `resolve_command_fqn`, which calls `resolve_command_with` directly, oracle
  = a closure over the live `commands: HashMap`. This is the hot dispatch
  path (`dispatch_words`, called from every `INVOKE_STK*`/`INVOKE_EXPANDED`
  opcode) — confirmed **no caching, no inline cache**, and
  `resolve_command_with` builds its full candidate `Vec<String>` eagerly
  before probing existence, so every non-absolute dispatch pays full
  candidate-list construction cost unconditionally. Worth heeding before
  routing more work (e.g. a future variable-resolution unification) through
  anything shaped like this without first adding a fast path.
- **WASM runtime** (`runtime/rust/src/namespace.rs`): `home_of` is a
  genuinely separate walk (not a literal reuse of
  `command_resolution_candidates`) but a verified structural mirror,
  walking namespace-tree bases directly with **no string allocation at
  all** — actually a cheaper shape than the VM's. If a shared abstraction
  for "resolve a name over some existence oracle" is ever built, it should
  probably be an iterator over borrowed base-handles rather than owned
  `String`s, so neither backend regresses.
- **Import semantics divergence is real and located on both sides**: WASM's
  `namespace import` installs a by-name redirect re-resolved fresh every
  call (so a source rename is followed, matching the pinned contract);
  the VM's `import_commands` instead clones the resolved `Command` value
  at import time (so a later redefinition of the source is not seen) — both
  exactly as the contract doc claims, now with exact file:line confirmation.
- **eBPF backend**: confirmed genuinely not a candidate for unification —
  no `tcl_syntax::naming` import anywhere in it; command dispatch is a
  plain compile-time Rust `match` over a fixed 24-verb set, no runtime
  table, no namespaces. Nothing to do here.
- **Codegen has zero static binding on both paths, but for two different
  reasons — a correction to `command-resolution.md`'s current wording.**
  The bytecode path is as documented: every proc call always emits runtime
  `INVOKE_STK*`/`INVOKE_EXPANDED`, no alternate direct-call opcode exists.
  The WASM path *also* never statically binds, but not because it "inherits
  the VM's conformance" (there is no VM involved) — it boxes every leaf
  command as literal source text and hands it to an imported host function
  that calls back into the **WASM runtime's own** `eval_str`/`home_of`
  dispatch. Should reword that line of the contract to say the WASM
  backend inherits the *runtime's* conformance via full eval-delegation,
  not the VM's.
- **Optimizer/interprocedural resolution confirmed to have zero codegen
  effect**: `resolve_internal_call`/`resolve_call_target` are consumed only
  by taint analysis; O103's `resolve_proc_qname` feeds only a source-text
  rewrite *suggestion* applied by a separate pre-compile pass, never an
  IR-to-codegen binding channel. No hidden divergent resolution affecting
  what actually gets emitted.

## Recommended sequencing (supersedes the companion doc's list for phases 1–2)

1. **Part A fix** — route Tier 1/2 target-selection sites through
   `resolve_called_proc` (or a class analogue). Fixes live silent-
   correctness bugs in Rename, Call Hierarchy, and Linked Editing Range.
   No new architecture, smallest possible diff for the risk it removes.
   Given the severity (renaming the wrong symbol, live-linking an unrelated
   call site), this plausibly deserves to be scoped and shipped
   independently of everything else here, on its own timeline.
2. **Part D fix** — collapse the three static class-resolution
   implementations onto the VM-verified one-hop rule. Fixes the confirmed-
   wrong MRO/superclass bug, removes two more duplicate implementations.
3. **Companion doc phase 1** — workspace-scoped `exists` oracle for
   cross-file reference enumeration. Fixes #923's confirmed regression.
4. **Companion doc phase 3** — TclOO method cross-file, reusing
   `class_lattice.rs`'s design (now sitting on the corrected class-name
   resolution from step 2).
5. **Part C spike** — scope variable-resolution consolidation properly
   before committing to an approach; the backends' data-model differences
   make this qualitatively harder than the command-name case.
6. **Companion doc phase 4** — SCCP-backed command-name-in-variable
   resolution.
7. **Companion doc phase 5** — lazy library/package resolution tier.

## Process: why the existing discipline didn't catch this, and what would

`command-resolution.md`'s own rule — "adding a resolution behaviour = adding
a vector" — is real and works for the consumers already inside the tent. It
did nothing for the ~17 that were never brought in, because there's no
mechanism that *notices* a new hand-rolled `.iter().find(name-equality)`
scan appearing outside the sanctioned helpers; the conformance suite only
gates code that already opted in. Worth considering, once Part A's fix
lands: a structural check (an `xtask` lint over the diff, or a `deny.toml`-
style pattern ban) that flags new bareword/simple-name matching over
`all_procs`/`all_classes` outside `tcl_syntax::naming` and the sanctioned
LSP-side helpers — turning "please call the shared function" from a
convention into something CI actually enforces, since the convention alone
already failed to catch this once.
