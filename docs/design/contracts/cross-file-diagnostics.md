# Contract: cross-file diagnostic authority and source advice

How a diagnostic about a **command** or a **package** takes account of facts
that live in another file: which lookup answers the question, what the
`source` graph contributes, and — the part that matters most in practice —
exactly when the server declines to answer at all.

Companion documents: [command-resolution.md](command-resolution.md) describes
selected command-name purposes and the Logical `Tcl_FindCommand` model;
[workspace-indexing.md](workspace-indexing.md) is what the index holds;
[lsp-diagnostics-publication.md](lsp-diagnostics-publication.md) is how a
diagnostic reaches the client. This document is the layer between them.

## Diagnostic authority and navigation candidates

Original C Tcl, Jim and hosted command diagnostics retain their independently
selected name purpose and current invocation owner. A sibling source header,
a reporting qualified name or a navigation candidate does not establish that
the callable is loaded at that site. Navigation may therefore offer a source
declaration while an unresolved-call diagnostic remains valid.

`AnalysisResult::allows_lexical_declaration_advice` selects the explicit Logical
compatibility domain. Its shared input classifier also recognises retained
hosted policies and contexts when the occurrence inventory is empty; changing
a reporting label cannot enable the Logical branch. In that branch,
`settle_call_against_workspace` (`tcl-lsp-server/src/lib.rs`) is the common
report-index settlement used by workspace diagnostic and navigation helpers.
Original declaration navigation uses current source/URI/configuration/Registry
owners separately. Reporting arity and candidate sets cannot replace an absent
Original callable receipt.

For example, a workspace containing `proc libtest {a b c} {…}` and a sibling
`libtest 1 2` call can provide source navigation. Explicit Logical settlement
also uses that reporting signature for cross-file arity. Original dispatch
requires its own loading and current callable provenance before that signature
can govern the invocation.

The Logical compatibility lookup applies these rules, in this order:

1. A live `namespace import -force` has *replaced* the importing namespace's
   own command of that name, so **no** candidate settles the call — it
   reaches the import's source instead (issue #1103).
2. Candidates are tried in **`Tcl_FindCommand` priority order** — the order
   `finalise_invocation_resolutions` recorded for that exact site.
3. A candidate naming a real registry builtin counts a proc definition only
   when that definition is not itself nested inside another proc's or class's
   body. The "rename the builtin away, install a same-named shadow, restore
   it" idiom must not make the shadow permanently outrank the builtin.
4. A name this document gains only from a `rename` / `interp alias` written
   *after* the call is not a command there yet (tclsh: `invalid command
   name`), so a workspace link cannot settle it (issue #1064).
5. Otherwise the candidate settles if this document defines it, or the
   workspace does.

### Logical qualified candidates and the optional bare-tail tier

The common Logical tier compares fully qualified reporting candidates.

A bare `current_class` called from namespace `::foo` has candidates
`::foo::current_class` and `::current_class`. A `proc
::clay::define::current_class` defined in some other file matches *neither*,
so its W123 stands — correctly, since Tcl would never route that call there.
A bare-tail match silences most of a large library's W123s, many of them with
no resolution candidate at the call site to justify it, so that tier stays
opt-in behind `tclLsp.features.crossFileResolution`; this one does not need
to be, because its suppression is restricted to the same Logical reporting
candidate set. Original navigation alone supplies no diagnostic suppression.

## Cross-file arity (E002 / E003)

In the explicit Logical branch, once a call settles on a workspace proc, its
argument count is checked against that proc's reported `(min, max)` envelope, reported with the analyser's
own codes, message shape, severity and disable filter — a cross-file arity
problem is the same defect as a same-file one and is classified identically.

The envelope comes from `WorkspaceProc::arity`, which is
`tcl_compiler::analyser::ProcDef::arity` — so `args` tails, defaults, and
computed parameter lists are represented by that reporting schema. Original
formal topology and effective original argv retain their separate purpose; an
index envelope supplies no entered binder, frame or native compilation fact.
`param_count` is *not* usable for this: it is the raw formal count and says
nothing about defaults or `args`.

### Where arity abstains, and why

| Situation | Behaviour | Why |
|---|---|---|
| `proc p {a args}` | no upper bound enforced | a trailing `args` accepts any count above the minimum |
| `proc p {a {b 2}}` | minimum lowered to 1 | a default makes the parameter optional |
| `proc p $params {…}` | fully open `0..∞` | a computed parameter list declares an unknown number of formals; reading the empty recorded list as "takes no arguments" drew a false E003 in issue #1107 |
| the name is also a class | resolved, never arity-checked | the call may dispatch to the arity-less class command |
| the name comes from `interp alias` / `rename` / `namespace import` | resolved, never arity-checked | the call reaches a different command, and an alias may bind leading arguments, shifting the count the callee sees |
| `p {*}$args` | skipped | the true argument count is a runtime fact |
| a command-prefix callback head | skipped by the direct check | it is not literally invoked with N arguments at that span |
| several procs share the qualified name | union of their envelopes | a count fitting *any* of them is not an error |

## The `source` graph and package requirement advice

`source FILE` runs `FILE` inline, in the caller's namespace, **at that
statement**. The workspace graph carries package requirement advice in both
directions;
it does not itself certify execution of the edge. The directions are not
symmetric:

* **Down** (`ancestor_requires`, issue #804) — *what did my callers already
  load before running me?* A module `source`d by an entry file that required
  `Tk` may use `winfo` with no `package require` of its own. This is ambient
  over the whole module under the graph's caller-prologue assumption; no
  position in the module's own text gates that inherited advice.
* **Up** (`descendant_requires`, issue #1332) — *what did the files I
  `source` load on my behalf?* `source tkFile.tcl`, where `tkFile.tcl` does
  `package require Tk`, contributes positioned `Tk` requirement advice in the sourcing file.
  Actual provision and loader completion remain separate. This one **is** positioned.

Both live in [`tcl_lsp_core::source_graph`](../../../rust/tcl-lsp-core/src/source_graph.rs).
`OriginalSourceInheritance` retains `PackageRequirementAdvice`, including the
original package key, requirements and package preference. W120 refinement
uses those keys; Original W123 refinement consumes the structured unresolved
command subject and package resolver's purpose-specific command query.
Reporting names, project tails and math-function labels cannot satisfy that
query. A file candidate or requirement is still source advice, not proof of a
running package provision or a loaded callable.

### Position matters — verified against C Tcl 9.0.4

```tcl
# child.tcl
package require msgcat
```
```tcl
# t1.tcl
puts [expr {[lsearch -exact [package names] msgcat] >= 0}]   ;# 0  — absent
source child.tcl
puts [package present msgcat]                                ;# 1.7.1 — present
```

So a `winfo` written *above* the `source` still draws W120, and one written
below it does not. Order-gating uses
`tcl_compiler::analyser::indirection::in_effect_within`, the same primitive
the single-document tiers use, which also gets the proc-body case right: a
load-level `source` contributes advice for a call written inside a proc body
under the reporting model that the module loads before those bodies run. A future source header or a
conditional graph edge does not establish actual Original execution ordering.

### Source path candidates

| Written as | Followed? |
|---|---|
| `source lib.tcl` (literal, relative or absolute) | yes |
| `source [file join [file dirname [info script]] lib.tcl]` | yes — statically folded; this is the idiom real projects use |
| `set p lib.tcl; source $p` | yes when the value constant-folds within the file |
| `source $somethingUnknowable` | no — abstains (below) |
| `source -encoding utf-8 lib.tcl` | only when the retained `SourceFileGrammar` admits that form: C8.5+ supports it; C8.4/Jim filename-only grammar declines |

The original source path comes from the selected source hook, original operand
and retained full source grammar/context. Known custom handlers, stale owners
and unavailable source values cannot borrow a nominal `source` role. Produced
loader scripts lacking a genuine script receipt remain directory or deferred
text advice rather than source navigation.

The table describes representable path candidates, including the explicit
Logical constant-folding branch. Path resolution is lexical (`.` and `..`
folded without touching the filesystem, so no symlink is ever followed), and the resolved child must be a
document the workspace index holds.

## Abstention is an answer

The failure mode this whole area guards against is a **false positive on a
real user's project**. Where a fact is not provable, the server declines to
claim it. Two mechanisms, at two levels.

### 1. `has_dynamic_providers` — the analyser's file-wide widening

The Logical reporting analyser sets this when the document makes its
reporting command set unknowable, and widens its W120/W123 advice:

- `load`, an `auto_path` mutation, a dynamic `package require` name;
- `namespace unknown HANDLER` — the handler runs for every failed lookup and
  may resolve anything;
- a dynamic `namespace import` pattern;
- a genuinely dynamic `rename`;
- **a `rename` or `interp alias` that moves any `LOADS_EXTERNAL_UNIT`
  command** (`source`, `load`, `auto_load`) out from under its own name.
  Hook dispatch keys off the *written* head, so once `source` has been
  renamed the files it pulls in are invisible; rather than confidently report
  a package missing that the moved command loads, the server widens. The test
  is the registry trait, so no command name appears in the analyser and a
  dialect that adds another file-loading command is covered by declaring it.

The Logical unresolved-site inventory is empty in these
states, and when the document has any `package require`, and when a user
`proc unknown` has a dynamic dispatch shape. Cross-file W123 suppression and
cross-file arity are both driven off that same list, so they abstain together
and by construction — not by two separately-maintained rules that could drift.

### 2. `SourceInheritance::unresolvable_source` — the server's path-level widening

Set when a `source` statement in this document cannot be pinned to an indexed
document: a path no static fold can prove, a file outside the workspace, or
one that does not exist. That file may `package require` anything and define
any command, so W120 and W123 abstain document-wide.

Following the `source` is the first choice, because it preserves the
diagnostics where the server can be sure; abstention trades a false positive
for a false negative only in the genuinely unknowable case — which is the
right way round.

### Source graph limits

These are widening choices in workspace source/package advice. They do not
turn a graph candidate into a native dispatch receipt or suppress an Original
diagnostic through the guarded reporting settlement:

- **A conditional or proc-body `source`.** `if {0} { source x.tcl }` and
  `proc load_it {} { source x.tcl }` are followed like any other. The graph
  has no conditional-execution analysis, and treating "might not run" as
  "definitely did not run" is what produced the reported false positive.
- **Transitive depth.** A package required below the first hop is attributed
  to the outermost `source` statement's position without proving that the
  inner hops execute.
- **A cycle or a file sourced twice.** Both terminate cleanly; a twice-sourced
  file simply contributes at both positions.

### What still reports correctly, and should

- **A safe interpreter.** `$safe eval {source x.tcl}` does not suppress
  anything, because a safe interp has no `source` — W129 says so, and the
  W120 that follows is correct. Verified: tclsh 9.0.4 answers
  `invalid command name "source"` in a `-safe` child.
- **W300**, the dynamic-`source`-path security warning, is orthogonal and
  unaffected. It keeps firing on the computed form even when the path folds
  successfully and the `source` is followed.

## Cost

Nothing here runs unless it can change a verdict. The `source` inheritance is
computed only when the document has a W120 or W123 to refine; call settling
only when the analyser recorded an unresolved site; the workspace name memo
only when a tier that needs it applies. The two index reads share one lock acquisition. Original and Logical
refinement retain their independently selected advice domains.

## Test anchors

| What | Where |
|---|---|
| the `source` graph's up direction, in isolation | `tcl-lsp-core/src/source_graph.rs` (`descendant_requires_*`) |
| Logical settling/arity, structured Original subjects and advice-domain guards | `tcl-lsp-server/src/lib.rs` unit tests |
| Logical two-file shapes over the real protocol | `tcl-lsp-server/tests/e2e/issue1331_crossfile_diagnostics.rs` |
| the Problems-panel outcome | `editors/vscode/src/test/crossFileDiagnostics.test.ts` |

The [workspace diagnostic refinement proof](../analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md)
binds the reporting-domain guards. [Package file provenance](../analysis/name-resolution-proofs/original-package-file-candidate-provenance.md)
and [source reachability controls](../analysis/name-resolution-proofs/original-package-reachability-source-controls.md)
cover the separate original source-advice producers. Linked tests are coverage
obligations; their presence does not claim execution.
