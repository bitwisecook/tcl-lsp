# Interprocedural call-site literal evidence

The compiler records, for each procedure, the literals this file's calls (and
a host's workspace view) pass at each parameter position. That evidence is
advice only. It never seeds the procedure body's lattice: a body is analysed
from a generic entry, each formal unknown, because a procedure stays callable
from a later `source`, a callback or an interactive session, which no
enumeration of call sites bounds. tclsh 8.4 to 9.1: with
`proc helper {mode} {if {$mode eq "prod"} {set x 1} else {set x 2}}` and a file
whose only calls are `helper prod`, a later `list [helper prod] [helper dev]`
is `1 2`.

No proof consumer reads the evidence: not the strict SCCP lattice, the
optimiser's rewrites, codegen, or body diagnostics such as `I230`. A display
may say what the file's calls pass ("every call in this file passes `prod`"),
worded as advice, from the evidence itself. A call site may still fold (`O103`)
through its own re-run with the arguments that call passes; the body is never
rewritten.

The rest of this page is the enumeration contract the evidence keeps, so the
advice it gives is not wrong: a caller the scan fails to attribute must not
read as agreement.

## Ownership

| Concern | Current owner |
|---|---|
| Enumerating source call-site evidence | `unit_scope.rs::collect_call_site_constants` and `record_call_site_evidence` |
| Identifying candidate script/callback argument positions | `tcl-registry` (`ArgRole`, `Traits`) |
| Resolving an ordinary source call target | retained source binding, `NormalUserProcedureInvocation`, and `lookup_command_word` in the source realm |
| Callback target evidence | `original_source_callback_targets_for_tokens` and `record_original_callback_callers` |
| `rename` / `interp alias` trust | `command_binding::ModuleCommandMutations` |

Registry roles identify where a script or callback may be supplied; they do
not by themselves prove the runtime target. In particular, parsing the head of
a `CommandPrefix` value is not evidence that it was registered, invoked, or
resolved to a callable procedure. The scan uses original-source callback target
receipts when available and otherwise records the caller as unenumerable, which
withdraws affected seeds. Likewise, ordinary-call candidate names are not
callable identities: source-owned target and allocation evidence determine
which procedure body a caller can reach. `resolve_internal_call` is a test-only name-resolution helper.
`command_prefix_head` parses a literal Tcl list value, but does not establish
registration, invocation, target identity, or body entry; neither helper is the
production owner for these source receipts.

## The three kinds of caller

1. **A literal command word** — `helper prod`. Resolved in the *calling*
   body's own namespace, with a `namespace import` fallback. The bodies
   walked as callers are the top level, every procedure, every `TclOO`
   method, and every synthetic body unit (`apply` lambda, `namespace eval`
   block).
2. **A script nested in an argument** — `catch { helper dev }`, a literal
   `uplevel { … }`, a non-exact `switch` arm. Re-segmented and scanned in
   place, to `MAX_CALL_SITE_BODY_DEPTH`.
3. **An indirection** — a dispatched command word (`$cmd dev`), a callback
   prefix, a user-proc invoker, or a script the command receives as a
   *value* (`eval $script`, `apply $fn`).

## How an indirection is resolved

A dispatch word is resolved **by value**, never by consulting the SCCP
lattice this seed feeds:

- literal assignments in the dispatching body (`set cmd helper`) contribute
  their values;
- when the word names one of the body's own parameters, the literals its
  callers pass at that position contribute too — read from the evidence map
  the scan itself is building;
- everything else (a computed value, a namespace-qualified `$::cmd`, an
  array element, a parameter of a body whose callers are not tracked)
  contributes "unknown".

A word that resolves to a set of names is recorded as an **ordinary call
site for each** of them. That is what keeps `set cmd helper; $cmd prod`
folding while `set cmd helper; $cmd dev` stops folding — a module-wide "any
`$cmd` disqualifies everything" wildcard could not make that distinction.

### The fixpoint

Parameter value sets read the evidence, and the evidence records
dispatches resolved from those value sets. `collect_call_site_constants`
resolves the circularity with an optimistic fixpoint: start from "no
callers seen", re-derive the whole evidence set from the previous round's,
and stop when it stops growing (`MAX_FIXPOINT_ROUNDS`, currently 6, is a
defensive backstop, not the termination argument). Each round is monotone in its
input — values only union, unknown flags only set — so the chain increases
to a fixpoint at which the value sets and the evidence agree. A round runs
only when a value set was actually consulted, so a module with no
indirection costs exactly one walk.

### When nothing can be claimed

`CallSiteEvidence::opaque_callee` records that some call in the module may
reach *any* procedure with *any* arguments, and
`params_constants_from_call_sites` then returns `None` for every procedure.
It is set by an unenumerable dispatch word and by a script the scan cannot
read — one the command receives as a value rather than as text. The
discriminator for the latter is "the whole word is one substitution"
(`value_shapes::is_pure_var_ref` / `parse_command_substitution`), **not**
"the word contains a `$`": `catch {puts $x}` carries readable script text
and is still walked in place, whereas `catch $body` does not.

Two further whole-module gates withdraw every seed for the same
"completeness is unproven" reason: a `package provide` in the file (another
file may call these procedures) and a `rename` / `interp alias` touching the
callee's name (`trusts_proc_binding`). A frame-shifting `uplevel` body is
not a gate of its own: its writes land in a frame the per-scope variable
scan does not own, so every value a local holds becomes unenumerable to the
scan, while a literal seed survives it (`bump n` keeps `name = n`). The
seeds are withdrawn where a dispatch word such as `$cmd` then cannot be
enumerated, as an unenumerable dispatch word always withdraws them.

## Known residual gaps

- **Cross-file callers** of a plain (non-`package provide`) file that
  another file `source`s and calls differently.
- **`namespace ensemble configure -map`** redirection.
- **The `unknown` handler.** A call to a command that exists nowhere is
  routed by Tcl to `unknown`, with the failed word and its arguments. A
  module that both defines `proc unknown {cmd args}` *and* seeds another
  procedure would need those routed calls counted against `cmd`. Closing it
  needs a registry fact marking the unresolved-command handler.
  `namespace unknown` and `package unknown` handlers are already covered —
  the registry declares their handler argument `ArgRole::CommandPrefix`.
- **A computed head resolving to a variable-writing builtin** (`set cmd
  set; $cmd x 5`) — this would need a builtin's own name to be among the
  literals a local holds.
- **`uplevel #0` namespace attribution** — a bare command in an absolute
  `uplevel #0` body resolves against the global namespace, but
  `Statement::UpFrame` keeps its body as a nested `Script` the CFG does not
  flatten, so the call reaches this scan only through the enclosing `proc`
  statement's own body text, in the declaring namespace. Closing it needs
  the registry to say which argument of a frame-shifting command is its
  level. Pinned by
  `uplevel_zero_body_resolves_against_global_not_enclosing_namespace`.

## A call's effects on its caller

This page's seed runs one way, from every caller into the callee's own
lattice. The other way runs per call: a procedure's transfer summary
([interprocedural-analysis.md](interprocedural-analysis.md) § *Transfer
summaries*) says what a call does to the places its `Name` arguments name
and to the outer places it writes, and the caller's lattice applies it
where the call runs — at a call statement, and at a command a statement's
words run, through the statement's word effects: the definition point the
CFG builder puts right ahead of the statement and pairs with it by
construction (`<word-effects>`, `ssa::word_effects_host`), never by span. A
place takes what a re-run of the callee leaves in it, the callee's
parameters holding that one call's arguments and its links the caller's
places before the call, so `set n 1; bump n` leaves `n` at 2; a `Name`
parameter the call omits names the place its default spells, so after `set
n 1; bumpd` for `proc bumpd {{name n}} {upvar 1 $name v; incr v}` the
caller's `n` is 2. A call whose re-run under its seeds reaches no exit, or
whose word count the callee rejects, never completes normally, and where a
run reads its caller's exits it is a raise there. That value is
exact under one call only, so it never enters the callee's own lattice or
its summary, which this page's seed reaches only where every caller agrees.
One summary serves every caller: after `set n 1; bump n; set other 10; bump
other` the caller's lattice holds `n` at 2 and `other` at 11, each call
re-running `bump` under its own place, and `bump`'s body is left as written,
since a body specialised to one place is right only while that call stays
the only one — and a call a word nests (`set z [bump m]`) is a call site too
(#2134).

## Seeing it

The compiler explorer's **interprocedural** view reports, as advice, what
every call in this file passes:

```text
$ tcl explore --show interproc --text example.tcl
=== interproc ===
└── ::helper arity=1..1
    · arity: …
    · pure: …
    · foldable: …
    · return shape: …
    · calls: —
    · every call in this file passes: mode = prod
    · flags: —
```

The row is pushed only when the calls agree, and disappears when an
indirection reaches the procedure with a differing or unknown argument. It
never changes what the body's analysis proves.

## Tests

- `rust/tcl-compiler/src/unit_scope.rs::tests` — the evidence map, the
  per-scope variable facts, and the shape helpers.
- `rust/tcl-compiler/src/compilation_unit.rs::tests::call_site_param_constants`
  — the suite over namespaces, recursion, methods, imports, rename,
  dispatch, callbacks, and `package provide`; a body never folds a condition
  over a parameter, whatever the evidence says.
- `rust/tcl-lsp-server/tests/e2e/diagnostics.rs` — the same behaviour over LSP.
- `editors/vscode/src/test/issue969.test.ts`, `issue976.test.ts`.
- `rust/tcl-explorer/src/serialise.rs::interproc_view_shows_what_every_call_passes_and_its_withdrawal`.

## Related

- [sccp-core-analyses.md](sccp-core-analyses.md)
- [interprocedural-analysis.md](interprocedural-analysis.md)
- [fp-sweep.md](fp-sweep.md) — the false-positive sweep harness.
- [KCS: when is a proc parameter treated as a constant?](../../kcs/kcs-qa-when-is-a-proc-parameter-treated-as-a-constant.md)
