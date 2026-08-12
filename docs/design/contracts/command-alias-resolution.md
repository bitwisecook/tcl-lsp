# Command alias resolution — the static, editor-facing slice

How the analyser and the LSP answer "what does this written command name
actually denote?" when `rename` or `interp alias` has moved it. This is the
static counterpart of the runtime contract in
[command-binding-and-aliasing.md](command-binding-and-aliasing.md); the
resolution algorithm itself is [command-resolution.md](command-resolution.md),
and the `namespace import` half of the same question is covered there too.

## Why it exists

A Tcl command table is mutable at run time. `rename OLD NEW` moves a command;
`interp alias {} ALIAS {} TARGET` installs a second name that re-resolves to
`TARGET` on every invocation (silently replacing any existing command of that
name). A call site written after either statement therefore reaches a
*different* definition than its spelling suggests. Everything that answers
"what does this word name?" — the W307/W308 method checks, go-to-definition,
find-references, rename, call hierarchy — has to follow the same chain the same
way, or the providers disagree with each other and with `tclsh`.

Without it, an aliased command is treated as unknown and its arguments are not
analysed for variable references, expression bodies, or script bodies —
producing false positives such as W214 (unused parameter):

```tcl
interp alias {} = {} expr

proc calculate {x y} {
    set result [= {$x + $y}]   ;# $x and $y are reads, via the alias
    return $result
}
```

## One hop-walk

`tcl_compiler::analyser::indirection::walk` is the single implementation.
A command *name* is not a definition; it is a slot whose contents change over
the life of the script. Every statement that writes such a slot — `proc`,
`rename`, `interp alias` — is an event on that name's timeline, and the
question every consumer really asks is *"what does this name hold at this
point, in this execution context?"*. `walk` answers it for the two mutation
kinds; `AnalysisResult::proc_def_in_effect_at` answers the `proc` half under
the same order rules.

The records themselves live on the analyser and are mirrored onto
`AnalysisResult`, keyed by qualified name: `command_aliases` / `alias_offsets`
for `interp alias`, `renamed_commands` / `rename_offsets` for `rename`.

### The rules

- **Order-gating.** A hop counts only once the statement establishing it has
  run: textual order at top level, and — for a statement written *outside* the
  body now executing — unconditionally inside a proc or class body, because
  the whole file loads before any body runs. A mutation that is itself a
  statement *of the running body* stays order-gated by offset. Oracle
  (tclsh 8.6.14 / 9.0.4): with `proc greet {} {…}`, a `hello` written before
  `rename greet hello` raises `invalid command name "hello"`; written after,
  it returns `greet`'s body — while `greet` itself then raises
  `invalid command name "greet"`.
- **Latest binding wins.** A name may carry both a `rename` record and an
  `interp alias` record; the later-written one is what the slot holds. Oracle:
  `proc a …; proc b …; rename a x; interp alias {} x {} b; x` → `B`.
- **A rename moves the command object; an alias re-resolves by name.**
  `rename p oldp` hands `oldp` the *object* `p` held at the rename, so a later
  `proc p` does not change what `oldp` runs — oracle: with
  `proc p {} {return first}; rename p oldp; proc p {} {return second}`,
  `oldp` → `first` (and `oldp`'s arity is the *first* signature), while `p` →
  `second`. An `interp alias` looks its target up by name on every invocation,
  so it sees the table as it stands *at the call*. `Indirection::resolve_at`
  carries whichever as-of time applies, and consumers resolve the terminal name
  at that time.
- **Hop cap.** Eight hops (`MAX_COMMAND_NAME_HOPS`) — the same cap the
  user-call arity resolver applies to the identical chains, so a
  `rename a b; rename b c; …` cycle cannot spin.
- **Prepended arguments decline.** `interp alias {} Cat {} Dog extra` binds a
  leading argument, so `Cat …` is not the call `Dog …` would be (tclsh:
  `withextra x` fails `wrong # args: should be "withextra"`). Such a chain is
  declined outright rather than resolved to the target.
- **Self-alias decline.** An alias whose canonical target is its own name is
  not a hop.

## Consumers

Every consumer of the walk gets the same answer by construction:

- the W307/W308 method checks
  (`diagnostics::var_command::class_reachable_by_indirection`);
- the LSP navigation providers — definition, references, rename, call
  hierarchy;
- the class-factory lookup (`analyser::handlers::class_factory_for_command`),
  which falls back to the walk when the direct local-then-workspace lookup
  misses, so `rename ::R::M ::R::Mk` followed by `::R::Mk create ::R::W {…}`
  still manufactures the class recorded under `::R::M`;
- IR lowering, which resolves an alias when lowering a command — an `expr`
  alias with a single braced argument produces the same expression-AST IR node
  the real `expr` would, so SSA tracks the variable reads and W214 does not
  fire. When the alias prepends arguments, synthetic tokens keep the argv,
  texts, and word-shape arrays the same length.

## Supported forms

Only aliases in the current interpreter (empty source and target paths) are
tracked:

```tcl
interp alias {} = {} expr              ;# = is an alias for expr
interp alias {} myeval {} eval
interp alias {} myput {} puts stdout   ;# prepends "stdout"
```

An alias targeting a child interpreter (`interp alias child …`) is scoped to
that child's synthetic domain, not the parent's table — see
[command-resolution.md](command-resolution.md) under `interp eval` bodies.

Alias names are stored fully qualified (`::=`, `::math::=`) because
`interp alias` creates interpreter-wide commands. Resolution then follows the
ordinary command-lookup order, so a qualified call resolves directly and a bare
one tries the current namespace before the global one.

## Where it abstains

These are deliberate silences, not gaps to be papered over:

- **Dynamic alias names.** `interp alias {} $var {} expr` has no static name.
- **Dynamically loaded aliases** — created via `package require`, `source`,
  `auto_load`, or the `unknown` proc — are invisible to static analysis, and a
  document that does any of these widens (`has_dynamic_providers`, see
  [cross-file-diagnostics.md](cross-file-diagnostics.md)).
- **Cross-document ordering** beyond what the `source`/`package require`
  forest proves. Byte offsets order events only within one document.
- **A chain that cannot be ranked revokes nothing** — an unprovable removal
  abstains toward keeping the link.

## Key files

| File | Role |
|---|---|
| `rust/tcl-compiler/src/analyser/indirection.rs` | `walk`, `Indirection`, `LastHop`, `in_effect` / `in_effect_within`, `MAX_COMMAND_NAME_HOPS` |
| `rust/tcl-compiler/src/analyser/types.rs` | `command_aliases`, `alias_offsets`, `renamed_commands`, `rename_offsets`, `destroyed_commands` |
| `rust/tcl-compiler/src/analyser/handlers.rs` | alias/rename recording, `class_factory_for_command` |
| `rust/tcl-lsp-core/src/namespace_import.rs` | the `namespace import` sibling, under the same order rule |
| `runtime/rust/src/cmd_alias.rs` | the runtime's alias table |
