# Lowering dispatch — `arg_roles` and command classification

How a command is classified and routed to the IR node that fits it best, and
where to hook in lowering for a new command.

`_lower_command()` in `lowering/` dispatches each command through a hierarchy:
registered lowering hooks → match/case on command name → fallthrough via
`arg_roles`.  The dispatch produces specific IR nodes (`IRAssignConst`,
`IRAssignExpr`, `IRIf`, etc.) rather than generic `IRCall` wherever possible.

Source: `rust/tcl-compiler/src/lowering/`,
`compiler/lowering_hooks/`

## Content

### Dispatch hierarchy

```
_lower_command(cmd)
    │
    ├─ Check lowering hook on CommandSpec → spec.lowering(lowerer, cmd)
    │   (e.g. set → lower_set(), incr → lower_incr())
    │
    ├─ match cmd_name:
    │   ├─ "proc"     → extract params, lower body, register IRProcedure
    │   ├─ "when"     → lower iRules event handler body (indexed: ::when::EVENT#N)
    │   ├─ "if"       → _lower_if() → IRIf with IRIfClause list
    │   ├─ "for"      → _lower_for() → IRFor (init, cond, step, body)
    │   ├─ "while"    → _lower_while() → IRWhile (cond, body)
    │   ├─ "foreach"  → _lower_foreach() → IRForeach
    │   ├─ "catch"    → _lower_catch() → IRCatch
    │   ├─ "try"      → _lower_try() → IRTry with IRTryHandler
    │   ├─ "switch"   → _lower_switch() → IRSwitch with IRSwitchArm
    │   ├─ eval/uplevel/upvar → IRBarrier (defeats static analysis)
    │   │
    │   └─ default (fallthrough):
    │       ├─ arg_indices_for_role(BODY) → IRBarrier (has body args)
    │       ├─ arg_indices_for_role(VAR_NAME) → IRCall with defs
    │       └─ else → IRCall (generic)
```

### Lowering hooks — `lower_set()` example

`set` has a registered lowering hook (`rust/tcl-compiler/src/lowering_hooks.rs`).  It
pattern-matches on the second argument's token type:

| Token type of `args[1]` | IR node produced | Example |
|-------------------------|-----------------|---------|
| `STR` (braced string) | `IRAssignConst` | `set x {hello}` |
| `ESC` (decimal integer) | `IRAssignConst` | `set x 42` |
| `CMD` wrapping `expr` | `IRAssignExpr` | `set x [expr {$a + 1}]` |
| `VAR` or interpolated | `IRAssignValue` | `set x $y`, `set x "hi $name"` |
| 0 args (getter) | `IRCall` | `set x` (read variable) |

### Fallthrough with `arg_roles`

For commands not handled by hooks or match/case (e.g. `regexp`), the
registry's `ArgRole` annotations guide lowering:

```tcl
regexp {(\d+)} $input match submatch
```

```python
var_indices = arg_indices_for_role("regexp", args, ArgRole.VAR_NAME)
# → {2, 3}  (match, submatch)

IRCall(
    command="regexp",
    args=(r"(\d+)", "${input}", "match", "submatch"),
    defs=("match", "submatch"),   # SSA tracks these as definitions
)
```

### Barrier commands

Commands in `_DYNAMIC_BARRIER_COMMANDS` (`eval`, `uplevel`, `upvar`) default
to producing `IRBarrier`, telling all downstream passes to stop reasoning
about variable state at that point.

#### Barrier relaxation (static-body `eval` / `uplevel`)

A subset of `eval` and `uplevel` shapes is statically decidable from tokens
and lowers to richer IR nodes instead of the generic `IRBarrier`:

| Source shape | Lowered to | Gate |
|---|---|---|
| `eval {...}` (single braced-literal body) | `IRBlock` with `source_tokens.argv_texts[0] == "eval"` | body is `TokenType.STR` and contains no nested dynamic-shape barriers |
| `uplevel ?level? {...}` (static level, braced-literal body) | `IRUpFrame(frame_shift=..., body=IRScript(...))` | level is absent, a bare integer, or `#N`; body is `TokenType.STR` and contains no nested dynamic-shape barriers |

**Gate — statically decidable from tokens:**

1. Body argument's word-token type is `TokenType.STR` (braced literal).
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
is unchanged from the pre-relaxation barrier path: both `IRBlock`
(eval-shape) and `IRUpFrame` are treated as barriers by every analysis
pass (memory-SSA, var-escape, interprocedural, SCCP, load-forwarding).
The codegen benefit is avoiding a `tcl_eval` string round-trip when the
body is inlined directly. The runtime-level win (caller-local visibility
for `uplevel 1 {...}`) requires follow-up work that either inlines the
callee entirely or pushes real frames on proc-to-proc WASM calls — both
out of scope for the first relaxation wave.

**Follow-up: `uplevel`-passthrough inlining.** When proc B's body is
essentially `uplevel 1 {body}` plus trivial prologue/epilogue, a caller
A that calls B can inline B's body directly and collapse the
`IRUpFrame(frame_shift=1)` into `IRBlock(shift=0)` — no frame
manipulation needed because there is no frame boundary to shift across.
The heuristic is: callee body is a single `IRUpFrame(shift=1)` plus at
most literal parameter setup, callee is not recursive, and size(body_IR)
is under a small budget (or single call site). This is a distinct
optimiser pass, enabled by IRUpFrame's existence but not implemented in
the first relaxation wave.

**Implemented: `eval [list …]`** expression forms. Also
statically decidable when the inner `list` command's arguments
are all plain literals (`TokenType.ESC` with no `$` / `[`, or
`TokenType.STR`). The gate synthesises the body by joining the
list arguments — `STR` tokens get re-braced — and lowers the
result as `IRBlock`. See `_Lowerer._eval_list_literal_body` in
`rust/tcl-compiler/src/lowering/`. Shapes that stay as `IRBarrier`:
`eval [foo …]` (inner command isn't `list`), `eval [list $v w]`
(dynamic substitution), and `eval [list {*}$args]` (`{*}`
expansion).

### Fallback-to-runtime pattern

Lowering hooks and codegen helpers are **intentionally conservative**: when
a hook encounters a construct it cannot safely specialise (e.g. `{*}`
expansion inside a structured command, or a `subst` template with
multi-character backslash forms like `\xHH` or `\uXXXX`), it returns
`None` or falls through to the generic `IRCall` / `IRBarrier` node. The
runtime interpreter handles the full Tcl specification; the compiler only
inlines what it can prove is safe.

Functions that return `None` to signal "I cannot handle this" (e.g.
`_parse_subst_template()` in `rust/tcl-compiler/src/codegen/helpers.rs`) are not
incomplete — they are conservative by design. Missing escape forms are an
optimisation limitation, not a correctness bug.

## Decision rule

- To add lowering for a new command: if it needs special IR, register a
  lowering hook on its `CommandSpec`.  If it just needs `defs` tracking,
  annotate its registry entry with `ArgRole.VAR_NAME` at the right indices.
- Commands with `BODY`-role args that are not explicitly handled produce
  `IRBarrier` — which is conservative but correct.
- Never add a match/case branch for a command that can be handled by a hook —
  hooks are more modular and testable.

## Related docs

- [Example 22 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-22-lowering-dispatch--arg_roles-and-command-classification)
- [GLOSSARY.md — IR](../../GLOSSARY.md#ir)
- [kcs-lowering-contracts.md](../../../docs/design/compiler/lowering-contracts.md)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
