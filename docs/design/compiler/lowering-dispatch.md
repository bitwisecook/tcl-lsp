# KCS: Lowering dispatch — `arg_roles` and command classification

## Symptom

A contributor needs to understand how a Tcl command is classified and lowered
into the correct IR node, or needs to add lowering support for a new command.

## Context

`Lowerer::lower_command` in `rust/tcl-compiler/src/lowering/mod.rs` dispatches
each command through a hierarchy, and every rung is **registry-driven** — there
is no command-name `match` anywhere in it. The dispatch produces specific
`Statement` variants (`AssignConst`, `AssignExpr`, `If`, …) rather than a
generic `Call` wherever possible.

Source: `rust/tcl-compiler/src/lowering/mod.rs`,
`rust/tcl-compiler/src/lowering_hooks.rs`,
`rust/tcl-compiler/src/lowering/hooks/`

## Content

### Dispatch hierarchy

```
Lowerer::lower_command(seg, namespace)
    │
    ├─ CommandRegistry::command_table_effect → feed the alias table
    │   (`interp alias {} name {} target`, static `rename old new`)
    │
    ├─ record_namespace_directives (`namespace import` / `export`)
    │
    ├─ if target.is_trace_visible() → lower_default (every command stays a
    │   runtime dispatch so an execution trace observes it)
    │
    ├─ try_lower_hook: resolve_invocation → semantics.lowering_hook
    │   → dispatch_lowering_hook (Expr, Return, Set, Incr,
    │     AppendOrLappend, Unset, Global, Variable, Upvar)
    │
    ├─ structured_expand_barrier: a structured form with `{*}` expansion
    │   → Statement::Barrier
    │
    ├─ try_dispatch_structured_hook: the 15 structured LoweringHookIds
    │   ├─ Proc          → lower params, lower body, register a Procedure
    │   ├─ When          → iRules event handler body (::when::EVENT#N)
    │   ├─ NamespaceEval → body unit
    │   ├─ If / Switch / For / While / Foreach / Lmap / ForeachLine
    │   ├─ Catch / Try / Dict / ArrayFor / Apply
    │   ├─ Eval         → try_lower_eval_static, else lower_default
    │   └─ Uplevel      → try_lower_uplevel_static, else lower_default
    │   (a form with a failed shape precondition returns None and falls
    │    through)
    │
    └─ lower_default (fallthrough):
        ├─ resolve_alias → canonical_command + prepended args
        ├─ arg_indices_for_role(ArgRole::Body)     → Statement::Barrier
        ├─ arg_indices_for_role(ArgRole::VarWrite) → Statement::Call with defs
        ├─ arg_indices_for_role(ArgRole::VarRead)  → Statement::Call with reads
        └─ else                                    → Statement::Call (generic)
```

Dispatching through the typed `LoweringHookId` rather than a bare name match
buys two things: canonical resolution via `CommandRegistry::resolve_call` /
`resolve_invocation`, so a future spec that aliases an existing form
dispatches correctly the moment its `lowering_hook` is stamped; and a single
canonical key that the downstream audit / LSP / compiler-explorer surfaces
consume.

### Lowering hooks — `lower_set` example

`set` carries `LoweringHookId::Set`, implemented by `lower_set` in
`rust/tcl-compiler/src/lowering_hooks.rs`. It bails to a generic `Call`
immediately on `{*}` expansion, on any arity other than two arguments, or when
the *name* word is not a compile-time literal (`set $x v` is a dynamic store).
Otherwise, for a single-token value word it pattern-matches on the value's
`ArgTokenKind`:

| `ArgTokenKind` of the value word | `Statement` produced | Example |
|----------------------------------|----------------------|---------|
| `Str` (braced string) | `AssignConst` | `set x {hello}` |
| `Esc` whose text is exactly the canonical decimal form | `AssignConst` | `set x 42` |
| `Cmd` wrapping `expr` | `AssignExpr` | `set x [expr {$a + 1}]` |
| `Var` or a multi-token interpolated word | `AssignValue` | `set x $y`, `set x "hi $name"` |
| one argument (getter) | `Call` | `set x` (read variable) |

The `Esc` case is deliberately narrow: `set arg 0005` and `set x " 5"` must
store their source spelling verbatim, so the fold only applies when the word
is byte-identical to the parsed decimal form.

### Fallthrough with `ArgRole`

For commands with no lowering hook (e.g. `regexp`), the registry's `ArgRole`
annotations guide `lower_default`:

```tcl
regexp {(\d+)} $input match submatch
```

`CommandRegistry::arg_indices_for_role(cmd, args, ArgRole::VarWrite)` returns
the indices of the `match` and `submatch` words, which become the `defs` of
the emitted statement:

```rust
Statement::Call {
    command: "regexp".into(),
    canonical_command: None,
    args: vec![r"(\d+)".into(), "${input}".into(), "match".into(), "submatch".into()],
    defs: vec!["match".into(), "submatch".into()],   // SSA tracks these as definitions
    ..
}
```

`ArgRole::VarRead` feeds `reads` the same way, and a spec carrying
`Traits::READS_BEFORE_WRITE` (`lset`, `lpop`, `ledit`) sets `reads_own_defs`
so dead-store analysis keeps the feeding assignment live.

### Barrier commands

A command whose registry spec exposes `ArgRole::Body` arguments that no hook
handles lowers to `Statement::Barrier` (reason `"unsupported body command"`),
telling all downstream passes to stop reasoning about variable state at that
point. `eval` and `uplevel` reach the barrier the same way, when their
static-body relaxation below does not apply.

#### Barrier relaxation (static-body `eval` / `uplevel`)

A subset of `eval` and `uplevel` shapes is statically decidable from tokens
and lowers to richer IR nodes instead of the generic `Statement::Barrier`:

| Source shape | Lowered to | Gate |
|---|---|---|
| `eval {...}` (single braced-literal body) | `Statement::Block` | body word is `TokenType::Str` and contains no nested dynamic-shape barriers |
| `uplevel ?level? {...}` (static level, braced-literal body) | `Statement::UpFrame { frame_shift, absolute, body, .. }` | level is absent, a bare integer, or `#N`; body word is `TokenType::Str` and contains no nested dynamic-shape barriers |

The entry points are `Lowerer::try_lower_eval_static` and
`Lowerer::try_lower_uplevel_static`; each returns `None` on a failed
precondition, and `try_dispatch_structured_hook` falls back to
`lower_default`.

**Gate — statically decidable from tokens:**

1. Body argument's word-token kind is `TokenType::Str` (braced literal).
   Anything else (`Esc`, `Var`, `Cmd`) stays on the barrier path.
2. The body does not contain a nested script evaluator whose own script is
   still dynamic. A braced `eval {uplevel 1 $x}` stays a barrier because
   the inner `uplevel` has a dynamic body. See
   `rust/tcl-compiler/src/lowering/mod.rs::body_has_dynamic_barrier`.
   Both halves of that question are registry answers, never a name test
   (issue #1055): a command is an evaluator when
   `CommandRegistry::invocation_traits` — which composes `spec.traits |
   sub.traits`, so the compound members `namespace eval`, `namespace
   inscope`, and `interp eval` resolve too — reports
   `Traits::EVALUATES_CODE`, and *which* words make up its script comes from
   `arg_indices_for_role(…, ArgRole::Body)` plus, for a
   `Traits::SCRIPT_CONCATENATES_ARGS` evaluator, every word after the first
   script word (they concatenate into the same script, so a dynamic tail is
   dynamic). An evaluator the registry exposes no script word for — a
   malformed `uplevel 1`, or a command-prefix evaluator like `coroprobe` —
   poisons the gate rather than being guessed at.
3. For `uplevel`, the *lowering hook* still requires a static level (absent,
   a bare integer, or `#N`) — `uplevel $lvl {...}` stays a barrier. The
   gate in (2) does not re-derive the level: it asks the registry which word
   is the script, so a dynamic level in a *nested* `uplevel $lvl {literal}`
   does not by itself poison an outer relaxation.

The gate is token-level. **No SSA or escape analysis is required** at
lowering time; we look only at the lexed word structure.

**Architectural win (IR-level):** downstream optimiser passes can inspect
the parsed body without re-parsing the source text. The runtime behaviour
is unchanged from the pre-relaxation barrier path: both `Statement::Block`
(eval-shape) and `Statement::UpFrame` are treated as barriers by every
analysis pass (memory-SSA, var-escape, interprocedural, SCCP,
load-forwarding). The codegen benefit is avoiding a string round-trip
through the evaluator when the body is inlined directly. The runtime-level
win (caller-local visibility for `uplevel 1 {...}`) requires follow-up work
that either inlines the callee entirely or pushes real frames on
proc-to-proc WASM calls — both out of scope for the first relaxation wave.

**Follow-up: `uplevel`-passthrough inlining.** When proc B's body is
essentially `uplevel 1 {body}` plus trivial prologue/epilogue, a caller
A that calls B can inline B's body directly and collapse the
`UpFrame { frame_shift: 1, .. }` into a zero-shift `Block` — no frame
manipulation needed because there is no frame boundary to shift across.
The heuristic is: callee body is a single `UpFrame { frame_shift: 1, .. }`
plus at most literal parameter setup, callee is not recursive, and the body
IR is under a small size budget (or there is a single call site). This is a
distinct optimiser pass, enabled by `UpFrame`'s existence rather than by the
relaxation itself; see `rust/tcl-compiler/src/inline_uplevel.rs`.

**Implemented: `eval [list …]`** forms. Also statically decidable when the
inner `list` command's arguments are all plain literals (`TokenType::Esc`
with no `$` / `[`, or `TokenType::Str`). The gate synthesises the body by
joining the list arguments — `Str` tokens get re-braced, so list
canonicalisation stays correct — and lowers the result as
`Statement::Block`. See `eval_list_literal_body` in
`rust/tcl-compiler/src/lowering/mod.rs`. Shapes that stay a
`Statement::Barrier`: `eval [foo …]` (inner command isn't `list`),
`eval [list $v w]` (dynamic substitution), and `eval [list {*}$args]`
(`{*}` expansion).

### Fallback-to-runtime pattern

Lowering hooks and codegen helpers are **intentionally conservative**: when
a hook encounters a construct it cannot safely specialise (e.g. `{*}`
expansion inside a structured command, or a `subst` template with
multi-character backslash forms like `\xHH` or `\uXXXX`), it returns
`None` or falls through to the generic `Statement::Call` /
`Statement::Barrier`. The runtime interpreter handles the full Tcl
specification; the compiler only inlines what it can prove is safe.

Functions that return `None` to signal "I cannot handle this" — e.g.
`parse_subst_template` in `rust/tcl-compiler/src/codegen/helpers.rs`, which
returns `Option<Vec<SubstPart>>` — are not incomplete; they are
conservative by design. Missing escape forms are an optimisation
limitation, not a correctness bug.

## Decision rule

- To add lowering for a new command: if it needs special IR, stamp a
  `LoweringHookId` on its `CommandSpec::lowering_hook` and implement the
  arm.  If it just needs `defs` tracking, annotate its registry entry with
  `ArgRole::VarWrite` at the right indices.
- Commands with `ArgRole::Body` args that are not explicitly handled produce
  `Statement::Barrier` — conservative but correct.
- Never add a command-name branch to the lowerer for something a registry
  hook ID can express — the hook table is the canonical key every downstream
  surface reads, and name matching bypasses alias resolution.

## Related docs

- [Example 22 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-22-lowering-dispatch--arg_roles-and-command-classification)
- [GLOSSARY.md — IR](../../GLOSSARY.md#ir)
- [kcs-lowering-contracts.md](../../../docs/design/compiler/lowering-contracts.md)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
