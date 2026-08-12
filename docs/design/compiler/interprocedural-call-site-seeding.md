# Interprocedural call-site literal seeding

The compiler binds a procedure parameter to a compile-time literal when
every caller passes the same value there, and hands that binding to the
callee's own [SCCP](../../GLOSSARY.md#sccp) run as `param_constants`. The
fold that follows feeds `I230`, the optimiser's `O101` constant-condition
suggestion, and `O107`'s unreachable-code suggestion.

The whole contract of that seed is one sentence: **it is sound only if the
scan enumerated every caller.** A caller the scan fails to attribute does
not merely go uncounted — it vanishes from the "every caller agrees"
evidence, so an absence of contradicting evidence reads as agreement. Every
bug in this area (issues #969, #976, #977, #978) has been an instance of
that single failure, reached through a different kind of caller.

## Ownership

| Concern | Owner |
|---|---|
| Enumerating callers and their literal arguments | `tcl-compiler/src/unit_scope.rs` |
| Turning that evidence into an SCCP seed | `unit_scope.rs::params_constants_from_call_sites` |
| Interning the seed into the memo key | `compilation_unit.rs::encode_param_constants` |
| Which argument is a script / callback / variable name | `tcl-registry` (`ArgRole`, `Traits`) |
| Namespace-relative command resolution | `interprocedural::resolve_internal_call` |
| Which command a callback prefix names | `interprocedural::command_prefix_head` |
| `rename` / `interp alias` trust | `command_binding::ModuleCommandMutations` |

No command name appears in the scan. Script bodies come from
`ArgRole::Body`, lambdas from `ArgRole::LambdaLiteral`, callbacks from
`ArgRole::CommandPrefix` (with `Traits::BUILDS_COMMAND_PREFIX` for the
`[list cb $x]` shape), user-proc invokers from `Traits::INVOKES_USER_PROC`,
and variable writes from `ArgRole::VarWrite` plus
`Traits::CREATES_SCOPE_ALIAS`.

The two questions this scan shares with the general call-graph builder —
"which procedure does this bare name reach" and "which command does this
callback prefix name" — are answered by `interprocedural`, not re-derived
here, so the two consumers cannot drift on a shape one of them learns to
handle (issue #978).

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
folding while `set cmd helper; $cmd dev` stops folding — a distinction the
first attempt at this (a module-wide "any `$cmd` disqualifies everything"
wildcard, PR #970, reverted) could not make.

### The fixpoint

Parameter value sets read the evidence, and the evidence records
dispatches resolved from those value sets. `collect_call_site_constants`
resolves the circularity with an optimistic fixpoint: start from "no
callers seen", re-derive the whole evidence set from the previous round's,
and stop when it stops growing (`MAX_CALL_SITE_SCAN_ROUNDS` is a defensive
backstop, not the termination argument). Each round is monotone in its
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

Three further whole-module gates withdraw every seed for the same
"completeness is unproven" reason: a `package provide` in the file (another
file may call these procedures), a `rename` / `interp alias` touching the
callee's name (`trusts_proc_binding`), and a frame-shifting `uplevel` body
(its writes land in a frame the per-scope variable scan does not own, so
every value set becomes unenumerable).

## Known residual gaps

- **Cross-file callers** of a plain (non-`package provide`) file that
  another file `source`s and calls differently.
- **`namespace ensemble configure -map`** redirection.
- **The `unknown` handler.** A call to a command that exists nowhere is
  routed by Tcl to `unknown`, with the failed word and its arguments. A
  module that both defines `proc unknown {cmd args}` *and* seeds another
  procedure would need those routed calls counted against `cmd`. Closing it
  needs a registry fact marking the unresolved-command handler (the `Traits`
  bitfield is currently full at 64 bits, so that is a widening, not a new
  flag). `namespace unknown` and `package unknown` handlers are already
  covered — the registry declares their handler argument
  `ArgRole::CommandPrefix`.
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

## Seeing it

The compiler explorer's **interprocedural** view reports each procedure's
`param constants` — the seed it was analysed under:

```
$ tcl explore --show interproc --text example.tcl
=== interproc ===
└── ::helper arity=1..1
    · calls: —
    · param constants: mode = prod
```

The line disappears when an indirection withdraws the seed, which makes the
difference between "every caller agrees" and "a call site could not be
enumerated" directly observable — the first thing to check when a condition
on a parameter folded and it should not have.

## Tests

- `call_site_scan.rs::tests` — the evidence map, the per-scope variable
  facts, and the shape helpers.
- `compilation_unit.rs::tests::call_site_param_constants` — the TP/FP/TN/FN
  suite over namespaces, recursion, methods, imports, rename, dispatch,
  callbacks, and `package provide`.
- `tcl-lsp-server/tests/e2e/diagnostics.rs` — the same behaviour over LSP.
- `editors/vscode/src/test/issue969.test.ts`, `issue976.test.ts`.
- `tcl-explorer/src/serialise.rs::interproc_view_shows_the_param_constant_seed_and_its_withdrawal`.

## Related

- [sccp-core-analyses.md](sccp-core-analyses.md)
- [interprocedural-analysis.md](interprocedural-analysis.md)
- [fp-sweep.md](fp-sweep.md) — the false-positive sweep harness.
- [KCS: when is a proc parameter treated as a constant?](../../kcs/kcs-qa-when-is-a-proc-parameter-treated-as-a-constant.md)
