# Contract: cross-file diagnostics — one lookup, and where it abstains

How a diagnostic about a **command** or a **package** takes account of facts
that live in another file: which lookup answers the question, what the
`source` graph contributes, and — the part that matters most in practice —
exactly when the server declines to answer at all.

Companion documents: [command-resolution.md](command-resolution.md) is the
name-resolution algorithm itself (`Tcl_FindCommand` order);
[workspace-indexing.md](workspace-indexing.md) is what the index holds;
[lsp-diagnostics-publication.md](lsp-diagnostics-publication.md) is how a
diagnostic reaches the client. This document is the layer between them.

## The rule: there is exactly one cross-document command lookup

**`settle_call_against_workspace`** (`tcl-lsp-server/src/lib.rs`) settles a
call site against the workspace, and **every** consumer calls it —
go-to-definition, find-references, and the diagnostics path alike.

That is not a stylistic preference. Issue #1331 is the bill for having had
two. Given two files in one workspace:

```tcl
# deflib.tcl
proc libtest {a b c} { return [expr {$a + $b + $c}] }
```
```tcl
# plaincaller.tcl
libtest 1 2
```

`textDocument/definition` on `libtest` resolved to `deflib.tcl` — the index
plainly held it — while `publishDiagnostics` reported `W123 Unknown command
'libtest'` and, having no signature to compare against, never arity-checked
the call. One server, two contradictory answers about one name, because
navigation consulted the workspace index and diagnostics consulted a
different, bare-tail name set that was off by default.

The lookup applies these rules, in this order:

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

### Why this can be on by default when the older tier cannot

Because it matches **fully-qualified candidates**, not bare tails.

A bare `current_class` called from namespace `::foo` has candidates
`::foo::current_class` and `::current_class`. A `proc
::clay::define::current_class` defined in some other file matches *neither*,
so its W123 stands — correctly, since Tcl would never route that call there.
Measured over tcllib 2.0 (790 files, 450 W123s), the older bare-tail match
silenced 396 of them, 197 of which no resolution candidate at the call site
justified. That tier remains opt-in behind `crossFileResolution`; this one
does not need to be, because **anything it suppresses is something
go-to-definition would have navigated.**

## Cross-file arity (E002 / E003)

Once a call settles on a workspace proc, its argument count is checked
against that proc's real `(min, max)` envelope, reported with the analyser's
own codes, message shape, severity and disable filter — a cross-file arity
problem is the same defect as a same-file one and is classified identically.

The envelope comes from `WorkspaceProc::arity`, which is
`tcl_compiler::analyser::ProcDef::arity` — so `args` tails, defaults, and
computed parameter lists are all already handled correctly at the source.
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

## The `source` graph — a two-way channel

`source FILE` runs `FILE` inline, in the caller's namespace, **at that
statement**. Facts therefore flow in both directions, and the two directions
are not symmetric:

* **Down** (`ancestor_requires`, issue #804) — *what did my callers already
  load before running me?* A module `source`d by an entry file that required
  `Tk` may use `winfo` with no `package require` of its own. This is ambient
  over the whole module: the ancestor ran its prologue before entering it, so
  there is no position in the module's own text to gate on.
* **Up** (`descendant_requires`, issue #1332) — *what did the files I
  `source` load on my behalf?* `source tkFile.tcl`, where `tkFile.tcl` does
  `package require Tk`, makes `Tk` present in the sourcing file from that
  statement onward. This one **is** positioned.

Both live in [`tcl_lsp_core::source_graph`](../../../rust/tcl-lsp-core/src/source_graph.rs).

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
load-level `source` counts for a call written anywhere inside a proc body,
because the whole file loads before any body runs.

### Which `source` paths are followed

| Written as | Followed? |
|---|---|
| `source lib.tcl` (literal, relative or absolute) | yes |
| `source [file join [file dirname [info script]] lib.tcl]` | yes — statically folded; this is the idiom real projects use |
| `set p lib.tcl; source $p` | yes when the value constant-folds within the file |
| `source $somethingUnknowable` | no — abstains (below) |
| `source -encoding utf-8 lib.tcl` | yes; the option word is located from `source`'s own registry `OptionSpec` list, not a hardcoded `-encoding` |

Path resolution is lexical (`.` and `..` folded without touching the
filesystem, so no symlink is ever followed), and the resolved child must be a
document the workspace index holds.

## Abstention is an answer

The failure mode this whole area guards against is a **false positive on a
real user's project**. Where a fact is not provable, the server declines to
claim it. Two mechanisms, at two levels.

### 1. `has_dynamic_providers` — the analyser's file-wide widening

Set when the document does something that makes its command set unknowable,
and it suppresses W120 / W123 for the whole file:

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

The analyser also records **no unresolved call sites at all** in these
states, and when the document has any `package require`, and when a user
`proc unknown` has a dynamic dispatch shape. Cross-file W123 suppression and
cross-file arity are both driven off that same list, so they abstain together
and by construction — not by two separately-maintained rules that could drift.

### 2. `SourceInheritance::unresolvable_source` — the server's path-level widening

Set when a `source` statement in this document cannot be pinned to an indexed
document: a path no static fold can prove, a file outside the workspace, or
one that does not exist. That file may `package require` anything and define
any command, so W120 and W123 abstain document-wide.

This is option (2) of issue #1332, and it is deliberately the *second*
choice: following the `source` (option 1) preserves the diagnostics where the
server can be sure, and abstention trades a false positive for a false
negative only in the genuinely unknowable case — which is the right way round.

### Known false negatives, accepted deliberately

These are cases where the server stays quiet and a stricter analysis could
have spoken. Each is a considered trade, not an oversight:

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
only when a tier that needs it applies. A document with none of those pays
exactly what it did before any of this existed, and the two index reads share
one lock acquisition.

## Test anchors

| What | Where |
|---|---|
| the `source` graph's up direction, in isolation | `tcl-lsp-core/src/source_graph.rs` (`descendant_requires_*`) |
| settling, arity envelopes, abstention, position gating | `tcl-lsp-server/src/lib.rs` unit tests |
| both issues' two-file shapes over the real protocol | `tcl-lsp-server/tests/e2e/issue1331_crossfile_diagnostics.rs` |
| the Problems-panel outcome | `editors/vscode/src/test/crossFileDiagnostics.test.ts` |
