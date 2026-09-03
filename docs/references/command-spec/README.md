# The CommandSpec reference

> **Audience:** authors of Tcl libraries describing their commands, and
> contributors reviewing specs. For the guided workflow, start with
> [creating a command spec without knowing Rust](../../kcs/kcs-howto-create-command-specs-without-rust.md).
> The authoring format is a **SpecTcl** `.tclspec` pack — see [how to write
> one](../../kcs/kcs-howto-write-a-tclspec-pack.md) — alongside the
> [Spec Studio](https://bitwisecook.github.io/tcl-lsp/spec-studio/) for
> browsing the live registry and rendering a `.rs` module or a stub.

A `CommandSpec` is everything tcl-lsp knows about one command. The tools
never special-case command names: whatever `foreach` or `string` get, they
get by declaring facts in a spec — so your command can get the same by
declaring the same facts.

This manual is in two parts:

- **[fields.md](fields.md)** — every field of `CommandSpec` and
  `SubCommand`, with its meaning in Tcl terms and the full vocabularies
  (traits, argument roles, taint colours, …). Generated from the Spec
  Studio's own schema, so it cannot drift from the registry.
- **This page** — how a spec resolves at a call site, and the impact
  tables: which fields drive which editor features, optimisations, and
  diagnostics.
- **[The callback-surface inventory](../../design/contracts/callback-surface-inventory.md)**
  — every script, callback, and command-reference position the registry
  declares, projected into
  [`callback-surfaces.md`](../../generated/callback-surfaces.md), plus the
  authored [coverage manifest](callback-surface-requirements.json) that pins
  each documented callback to its classification, over a
  [baseline](callback-surface-baseline.json) that pins the rest by existence.
  Declaring a `-command` option or a command-prefix argument means adding a row
  to one of them — the gate rejects a callback surface no tier accounts for;
  the contract page has the checklist.

## How a spec resolves at a call site

A call is resolved once against the command, the matched subcommand, and
the matched form, and every consumer reads the combined answer
(`tcl_registry::resolved_invocation`). Two rules apply, and they differ:

- **Traits union.** The command's trait bits are OR-ed with the resolved
  subcommand's (a subcommand's `pure: true` folds in as `PURE` here). A
  subcommand can only ever *add* behaviour — it cannot mask its parent's
  traits. The legacy purity classifier is the one place
  `SubCommand::mutator` downgrades a pure parent.
- **Descriptors are most-specific-wins.** `result_stability`,
  `representation_effect`, `completion`, and the hook IDs take the form's
  value if set, else the subcommand's, else the command's. `arity`,
  `return_type`, `var_write_typing`, and `body_kind` take the
  subcommand's whenever a subcommand resolved at all. `side_effects`
  takes the subcommand's only when non-empty. `world_effects`,
  `state_transitions`, and `dispatch_dependencies` *compose* across all
  three levels rather than replacing. `frame_effect` is command-only.

Argument roles funnel through one query
(`CommandRegistry::arg_indices_for_role`): a resolved subcommand's
resolver-or-table wins outright (indices counted after the subcommand
word), a resolver *replaces* the static table at its level, and
`repeated_args` plus option-value roles union in additively.

**Unset always means unknown, never safe-to-assume.** Every optimiser
descriptor left `None` resolves to the conservative answer (no folding, no
reuse, a world barrier). Declaring facts only ever *adds* precision; a
command absent from the registry entirely is treated as an unknown
read-write of everything.

## What to declare first

For a third-party library, ordered by payoff:

1. **Exist at all.** Registration alone removes the unknown-command
   diagnostic, stops your calls poisoning every enclosing proc's purity,
   and narrows the optimiser's dispatch barriers.
2. **`arity` + `arg_roles`** — argument-count checking with your own
   synopsis in the message; script bodies highlighted, folded, and
   analysed as code; variable arguments joined to rename, references, and
   read-before-set analysis.
3. **`hover` + `options` + `subcommands` + `arg_values`** — hover,
   signature help, and completion. No `hover` means *no hover at all*.
4. **`required_package`, `dialects`, `lifecycle`** — availability done
   right: completion filtering, missing-`package require` fixes, and
   version gating.
5. **Traits + `side_effects` + taint fields** — the behavioural facts
   diagnostics and the optimiser key on. `PURE | CSE_CANDIDATE` is the
   opt-in to folding and hoisting; `EVALUATES_CODE` / `CREATES_BARRIER`
   is the deliberate opt-out that makes a call a hard barrier.

## Editor features by field

What each declaration buys in the editor (VS Code and every other LSP
client). "Nothing" below always means *for your command* — the rest of
the file is unaffected.

| Feature | Driven by | Left undeclared |
|---|---|---|
| Hover (command) | `hover` (`summary` + first `synopsis`), subcommand names, `required_package` | No hover at all |
| Hover (subcommand / option / method) | `subcommands[].hover`, `options[].detail`, `object_class.instance_methods[].detail` | Bare title line |
| Signature help | `hover.synopsis[0]` (parameters split from it); subcommand `synopsis`/`detail` | None |
| Completion: commands | `hover.summary`, `required_package` / `tcllib_package` (provenance label), `dialects` + `lifecycle` (filtering) | Listed with no docs, everywhere |
| Completion: subcommands, flags, values | `subcommands[]`, `options[]`, `arg_values`, option `values`, `versioned_arg_values` | Nothing offered |
| Highlighting inside body arguments | `ArgRole::Body` / `Expr` / `LambdaLiteral` (via `arg_roles` / resolver / `repeated_args` / option-value roles), `body_kind` | The braced word is one flat string — nothing inside is highlighted or tracked |
| Variables through your command | `ArgRole::VarWrite` / `VarRead` (positional or on option values) | Names paint as strings; invisible to rename and references |
| Callbacks as call sites | `ArgRole::CommandPrefix` via `command_prefixes` (+ resolver), `CommandName`, `Traits::BUILDS_COMMAND_PREFIX` | Callback names are inert data; call graph and "unused proc" miss them |
| Keyword / enum / pattern / format colouring | `ArgRole::Keyword`, `arg_values`, `pattern_type`, `format_string_type` | Plain strings |
| Folding | Body / lambda roles, `case_list`, `definition_body` | No fold regions |
| Go-to-definition, references, rename, code lens, call hierarchy | The same role set; `Traits::DEFINES_PROCEDURE`, `definition_body`, plain bodies | Sites reached through your command are invisible; rename silently misses them (a declared-but-computed name position makes rename refuse instead) |
| Outline / workspace symbols | `defines_symbol`, `definition_body`, `Traits::DEFINES_PROCEDURE` | Not listed |
| Class commands: `$obj m` completion, hover, colouring | `object_class`, `manufacturer_methods`, `binds_handle`, `creates_instance_at`, `definition_body` | Generic shape-based colouring only |
| Inlay hints (parameters, types, format specifiers) | `hover.synopsis`, `options`; `return_type` / `var_write_typing`; `format_string_type` + roles | None |
| Quick fixes | `required_package` (add the require), `lifecycle.deprecation_fix`, `deprecated_replacement` (+ `_drop_in`), `forms[].synopsis` | Diagnostic without a fix |
| Formatter layout | `arg_presentation` (`InlineScript` on a `Body` index), abbreviation expansion from `subcommands` / `options` / `prefix_matching` | Bodies block-format; no expansion |

Worth knowing: `CommandSpec::completion` is the *Tcl completion-code*
contract for the compiler, not LSP completion; `FormSpec` feeds only the
`usage:` suffix on arity diagnostics and one append-argument quick fix;
document links and the formatter's clause-list layout are name-hardcoded
(`source`, `switch`) rather than registry-driven today.

## Optimisations by field

The optimiser's O-codes and what enables or blocks each. The recurring
gate is the shared purity classifier: `Traits::PURE` (or
`PURE_EVALUATION`, or a pure resolved subcommand) enables;
`EVALUATES_CODE` / `CREATES_BARRIER` makes the call a hard barrier; an
**unregistered command blocks everything around it**.

| O-code | Rewrite | Registry levers |
|---|---|---|
| O100 constant propagation | value into uses | Needs typed IR from a `lowering_hook`; blocked by `ESTABLISHES_VARIABLE_TRACE`, `CREATES_SCOPE_ALIAS` |
| O102 load forwarding | literal into uses | Blocked by the same aliasing/trace traits and by any barrier statement |
| O103 pure-proc folding | call → constant | Every callee must classify pure — one unregistered command in a proc poisons it |
| O104 / O130 chain folds | `set`+`append`/`lappend` → one `set` | Escape gate is registry-driven; the heads themselves are name-hardcoded, so third-party RMW commands cannot join |
| O105 redundancy (GVN) | reuse a prior identical call | Legacy path: `PURE \| CSE_CANDIDATE`. The strict "Common" path additionally wants `result_stability: ReferentiallyTransparent`, `world_effects` / `state_transitions` declared empty — but its dispatch proof is deliberately unprovable today, so full-redundancy reuse never fires (test-pinned); partial redundancy and O106 use the legacy path and do |
| O106 loop hoisting (LICM) | invariant out of loop | `PURE \| CSE_CANDIDATE`, every nested substitution also pure — the most directly attainable win for a spec author |
| O107 / O112 unreachable + constant branches | dead blocks, `if {0}` | Need structured IR: only commands with a `lowering_hook` (`If` / `While` / `For` / `Switch`) participate; the hook enum is closed |
| O108 / O109 / O126 / O127 dead code, dead stores, inlining | removal / inlining | Purity classifier; `VarRead` / `WHOLE_ARRAY_ARG` roles prevent false "unused"; `READS_BEFORE_WRITE` protects RMW feeds; `DESTROYS_VARIABLE` keeps the killed store alive |
| O114 `incr` idiom | `set x [expr {$x+1}]` → `incr` | Needs the variable typed `Int` — via `return_type` / `var_write_typing` |
| O116 / O118 / O129 constant folds | `[cmd literal…]` → literal | **`const_fold` / `const_fold_versioned` — the single most actionable field**; module-wide trust is revoked by an unstamped command-table mutator |
| O111 brace-expr hint | pairs with W100 | `ArgRole::Expr` on the right argument — free once declared |
| O123 accumulator hint | recursion shape | `EXPR_CONCATENATES_ARGS` on the head |
| O124 unused iRules procs | comment out | Call-graph edges from `command_prefixes` / `INVOKES_USER_PROC` / body roles are what prevent **false** "unused"; any `EVALUATES_CODE` barrier suppresses the whole file |
| O101 / O110 / O113 / O115 / O117 / O119 / O120 / O125 / O128 | expression algebra, sinking | Mostly syntactic; O125 checks for `[` not purity; O128's index commands are name-hardcoded |

Compiler analyses read the same facts: type inference (`return_type`,
`var_write_typing`, `return_elements`), shimmer checks (`arg_types`,
`representation_effect`, `byte_array_effect`), taint (the `taint_*`
family plus `TAINT_SOURCE` / `TAINT_SINK`), memory and world-state SSA
(`world_effects`, `state_transitions` — where `None` means "assume it can
do anything"), scope analysis (`frame_effect`,
`StateTransition::VariableCellAlias`), and the CFG (`TERMINATES_BLOCK`,
`CATCHABLE_THROW`, `BREAKS_LOOP`, …). One caveat: the CFG terminator
classes and the frame-elision trait sets are built from the plain-Tcl
registry, so a dialect-only command does not participate in those two.

## Diagnostics by field

See [diagnostics.md](diagnostics.md) — one row per code a spec can
influence, with the field that causes or suppresses it.

## See also

- [The command registry design doc](../../design/compiler/command-registry.md)
  — the architecture and consumer contracts behind all of this.
- [The Spec Studio](../../kcs/features/kcs-feature-spec-studio.md) — edit
  every field here in a form, with this manual's text behind **?** buttons.
- [SpecTcl pack design](../../design/spec-packs.md) — the loadable
  command-pack architecture, discovery tiers, and crash containment, under
  active design for issue #1363.
- [How to write a SpecTcl pack](../../kcs/kcs-howto-write-a-tclspec-pack.md)
  — the quickstart: the minimal shape, the three discovery tiers, and how
  the running server picks a saved pack up.
- [How to derive version ranges from release history](../../kcs/kcs-howto-derive-version-ranges-from-releases.md)
  — deriving `introduced_version`/`retired_version` facts with `tcl spec
  import`.
