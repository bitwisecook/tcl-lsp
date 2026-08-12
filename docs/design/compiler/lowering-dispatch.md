# Lowering dispatch — `arg_roles` and command classification

How a command is classified and routed to the IR node that fits it best, and
where to hook in lowering for a new command.

`Lowering::lower_command` in `rust/tcl-compiler/src/lowering/mod.rs`
dispatches each command through a hierarchy: registered lowering hooks →
registry-driven structured-hook dispatch → fallthrough via `arg_roles`.  No
step matches on a command name: routing is by the typed `LoweringHookId`
the registry stamps on the `CommandSpec`.  The dispatch produces specific
`Statement` variants (`AssignConst`, `AssignExpr`, `If`, …) rather than a
generic `Statement::Call` wherever possible.

Source: `rust/tcl-compiler/src/lowering/mod.rs`,
`rust/tcl-compiler/src/lowering/structured.rs`,
`rust/tcl-compiler/src/lowering_hooks.rs`,
`rust/tcl-compiler/src/lowering/hooks/`,
`rust/tcl-registry/src/hooks.rs` (`LoweringHookId`)

### Dispatch hierarchy

```
Lowering::lower_command(seg, namespace)
    │
    ├─ Command-table bookkeeping: CommandTableEffect on the spec feeds the
    │   alias table (interp alias / rename), then namespace directives
    │
    ├─ try_lower_hook(...)  — the value-level hooks in lowering_hooks.rs
    │   (Set, Incr, Expr, Return, AppendOrLappend, Unset, Global, Variable,
    │    Upvar, Apply, ArrayFor)
    │
    ├─ structured_expand_barrier(...) — a {*}-expanded structured command
    │   cannot be specialised, so it becomes Statement::Barrier
    │
    ├─ try_dispatch_structured_hook(...) — registry LoweringHookId dispatch
    │   ├─ Proc          → extract params, lower body, register a Procedure
    │   ├─ When          → lower iRules event handler body (::when::EVENT#N)
    │   ├─ NamespaceEval → lower the namespace body
    │   ├─ If            → lower_if()      → Statement::If with IfClause list
    │   ├─ For           → lower_for()     → Statement::For (init, cond, step, body)
    │   ├─ While         → lower_while()   → Statement::While (cond, body)
    │   ├─ Foreach/Lmap/ForeachLine → lower_foreach() → Statement::Foreach
    │   ├─ Catch         → lower_catch()   → Statement::Catch
    │   ├─ Try           → lower_try()     → Statement::Try with TryHandler
    │   ├─ Switch        → lower_switch()  → Statement::Switch with SwitchArm
    │   └─ Dict / Eval / Uplevel
    │
    └─ lower_default(seg, namespace):
        ├─ arg_indices_for_role(ArgRole::Body) non-empty → Statement::Barrier
        ├─ arg_indices_for_role(ArgRole::VarWrite / VarRead) → Statement::Call with defs/reads
        └─ else → Statement::Call (generic)
```

A command absent from the registry, or whose `lowering_hook` is `None`,
falls through to `lower_default`.  Trace-visible compilation
(`CompileTarget::BytecodeTraced`) skips every hook and goes straight to
`lower_default`, so an execution trace observes each command as a plain
runtime dispatch.

### Lowering hooks — the `Set` hook

`set` carries `lowering_hook: Some(LoweringHookId::Set)`
(`rust/tcl-compiler/src/lowering_hooks.rs`).  The hook pattern-matches on
the second argument's token type:

| Token type of `args[1]` | Statement produced | Example |
|-------------------------|--------------------|---------|
| `STR` (braced string) | `Statement::AssignConst` | `set x {hello}` |
| `ESC` (decimal integer) | `Statement::AssignConst` | `set x 42` |
| `CMD` wrapping `expr` | `Statement::AssignExpr` | `set x [expr {$a + 1}]` |
| `VAR` or interpolated | `Statement::AssignValue` | `set x $y`, `set x "hi $name"` |
| 0 args (getter) | `Statement::Call` | `set x` (read variable) |

### Fallthrough with `arg_roles`

For commands no hook claims (e.g. `regexp`), `lower_default` consults the
registry's `ArgRole` annotations:

```tcl
regexp {(\d+)} $input match submatch
```

```rust
let var_indices = registry.arg_indices_for_role(&cmd, &args, ArgRole::VarWrite);
// → [2, 3]  (match, submatch)

Statement::Call {
    command: "regexp".into(),
    args: vec![r"(\d+)".into(), "${input}".into(), "match".into(), "submatch".into()],
    defs: vec!["match".into(), "submatch".into()],  // SSA tracks these as definitions
    ..
}
```

### Barrier commands

Commands whose registry `arg_roles` mark a `ArgRole::Body` word that no hook
specialises (`eval`, `uplevel`, `upvar`) default to producing
`Statement::Barrier`, telling all downstream passes to stop reasoning about
variable state at that point.  The choice is registry-driven — `lower_default`
tests `arg_indices_for_role(…, ArgRole::Body)`, never a command name.

#### Barrier relaxation (static-body `eval` / `uplevel`)

A subset of `eval` and `uplevel` shapes is statically decidable from tokens
and lowers to richer statements instead of the generic `Statement::Barrier`:

| Source shape | Lowered to | Gate |
|---|---|---|
| `eval {...}` (single braced-literal body) | `Statement::Block` | body is `TokenType::STR` and contains no nested dynamic-shape barriers |
| `uplevel ?level? {...}` (static level, braced-literal body) | `Statement::UpFrame { frame_shift, body: Script { … } }` | level is absent, a bare integer, or `#N`; body is `TokenType::STR` and contains no nested dynamic-shape barriers |

**Gate — statically decidable from tokens:**

1. Body argument's word-token type is `TokenType::STR` (braced literal).
   Anything else (ESC, VAR, CMD) stays on the barrier path.
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
is the same as the barrier path: both `Statement::Block` (eval-shape) and
`Statement::UpFrame` are treated as barriers by every analysis
pass (memory-SSA, var-escape, interprocedural, SCCP, load-forwarding).
The codegen benefit is avoiding a `tcl_eval` string round-trip when the
body is inlined directly. The runtime-level win (caller-local visibility
for `uplevel 1 {...}`) requires follow-up work that either inlines the
callee entirely or pushes real frames on proc-to-proc WASM calls — both
out of scope for the first relaxation wave.

**Follow-up: `uplevel`-passthrough inlining.** When proc B's body is
essentially `uplevel 1 {body}` plus trivial prologue/epilogue, a caller
A that calls B can inline B's body directly and collapse the
`Statement::UpFrame { frame_shift: 1, .. }` into a shift-free
`Statement::Block` — no frame manipulation needed because there is no frame
boundary to shift across.
The heuristic is: callee body is a single `UpFrame` with shift 1 plus at
most literal parameter setup, callee is not recursive, and the body IR's
size is under a small budget (or there is a single call site). This is a
distinct optimiser pass, enabled by `Statement::UpFrame`'s existence but not
implemented in the first relaxation wave.

**Implemented: `eval [list …]`** expression forms. Also
statically decidable when the inner `list` command's arguments
are all plain literals (`TokenType::ESC` with no `$` / `[`, or
`TokenType::STR`). The gate synthesises the body by joining the
list arguments — `STR` tokens get re-braced — and lowers the
result as `Statement::Block`. See `eval_list_literal_body` in
`rust/tcl-compiler/src/lowering/mod.rs`. Shapes that stay as
`Statement::Barrier`:
`eval [foo …]` (inner command isn't `list`), `eval [list $v w]`
(dynamic substitution), and `eval [list {*}$args]` (`{*}`
expansion).

### Fallback-to-runtime pattern

Lowering hooks and codegen helpers are **intentionally conservative**: when
a hook encounters a construct it cannot safely specialise (e.g. `{*}`
expansion inside a structured command, or a `subst` template with
multi-character backslash forms like `\xHH` or `\uXXXX`), it returns
`None` or falls through to the generic `Statement::Call` /
`Statement::Barrier`. The
runtime interpreter handles the full Tcl specification; the compiler only
inlines what it can prove is safe.

Functions that return `None` to signal "I cannot handle this" (e.g.
`parse_subst_template` in `rust/tcl-compiler/src/codegen/helpers.rs`) are not
incomplete — they are conservative by design. Missing escape forms are an
optimisation limitation, not a correctness bug.

## Decision rule

- To add lowering for a new command: if it needs a special `Statement`, set
  `lowering_hook: Some(LoweringHookId::…)` on its `CommandSpec` and add the
  matching arm.  If it just needs `defs` tracking, annotate its registry
  entry with `ArgRole::VarWrite` at the right indices.
- Commands with `ArgRole::Body` args that no hook handles produce
  `Statement::Barrier` — conservative but correct.
- Never dispatch on a command name in the lowerer — routing is by the typed
  `LoweringHookId` on the spec, which stays modular and testable.

## Related docs

- [Example 22 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-22-lowering-dispatch--arg_roles-and-command-classification)
- [GLOSSARY.md — IR](../../GLOSSARY.md#ir)
- [kcs-lowering-contracts.md](../../../docs/design/compiler/lowering-contracts.md)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
