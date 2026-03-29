# KCS: C++ command registry DSL

## Symptom

Adding new commands to the registry requires Python callables for
variable-layout commands.  The `dict[int, ArgRole]` representation
can't express unlimited tails or stride patterns, leading to hardcoded
index limits (e.g. `binary scan` stops at index 19).  Hardcoded command
knowledge is scattered across analyser, compiler, formatter, and LSP
features (~150+ references).

## Operational context

`native/include/tcl_lsp/registry/command_desc.hpp` defines the
`CommandDesc` struct and all supporting types.  This is the C++ DSL
that replaces `core/commands/registry/models.py:CommandSpec`.

`native/include/tcl_lsp/registry/command_registry.hpp` provides the
`CommandRegistry` class with binary-search lookup and `resolve_arg_roles`.

`native/src/registry/command_registry.cpp` implements role resolution,
layout resolvers, and hover rendering.

## Decision rules / contracts

1. **CommandDesc is the single source of truth** for command metadata.
   Never hardcode command names in the analyser, compiler, or formatter.
   Query the registry instead.

2. **ArgPattern replaces dict[int, ArgRole]**.  Use `TAIL` for unlimited
   trailing args, `STRIDE` for repeating groups, `FIXED` for positions,
   and `OPTION_VALUE` (or `OptionDesc.value_role`) for option semantics.

3. **LayoutResolver is a closed enum**.  Adding a new variable-layout
   command requires a new enum value and a C++ resolver function.
   This is intentional — variable-layout commands are rare (~12 total).

4. **Designated initializers are mandatory**.  Every `CommandDesc` must
   use C++23 designated initializers for readability.  Field order must
   match the struct declaration.

5. **Dialect filtering uses bitmask visibility**.  Don't use string
   comparisons for dialect checks.  Use `DialectFlags` and the
   `dialect_visibility()` expansion function.

6. **Per-subcommand options, not per-command**.  Each `SubCmdDesc` carries
   its own `options` span.  The terminator resolution checks the
   subcommand first, then falls back to the parent command.

7. **ArgTypeDesc is orthogonal to ArgPattern**.  Patterns describe
   semantic roles (what the arg *means*).  Types describe intrep
   expectations (what Tcl type the arg should *be*).

8. **HoverText is stored separately**.  Reference by `hover_index` into
   per-file `constexpr` arrays.  Don't bloat `CommandDesc` with large
   string literals.

9. **TclOO slot commands** use `is_slot_command = true` and carry the
   full slot operation option set (`-clear`, `-set`, `-append`, etc.).

10. **Proc shape inference** uses `resolve_arg_roles` generically.  The
    analyser never hardcodes command names for trait detection.  Any
    command in the registry automatically participates in trait inference.

## Gotchas

- `OptionDesc.value_hint` is for human display only; `value_role` carries
  the semantic information.
- `Arity.modulus = 0` means no modular constraint, not "any multiple of 0".
- `ArgPattern.index = -1` means "last argument" — only valid for `FIXED`.
- The `ResolveResult` uses inline storage for ≤16 args; overflows to heap.
  Most Tcl commands have <16 args, so heap allocation is rare.
- Layout resolvers that aren't yet implemented fall through to static
  pattern resolution.  This means `tcltest::test` uses patterns + option
  desc roles until its resolver is written.
