# Command registry DSL architecture

## Overview

The C++ command registry DSL replaces the Python `CommandSpec` system with a
declarative, `constexpr`-friendly data model.  Every command in every dialect
(Tcl 8.4–9.0, iRules, TMSH, Tk, Expect, EDA) is described by a single
`CommandDesc` struct that carries **all** metadata the analyser, compiler,
formatter, LSP features, and taint engine need.

**Design goals**:

1. Fix the `dict[int, ArgRole]` limitation — express "from index N onward,
   all args have role R" via `ArgPattern::TAIL`.
2. Express stride patterns, option-value roles, and variable-layout commands
   declaratively — no Python callables crossing the pybind11 boundary.
3. Use C++23 designated initializers for readable, self-documenting descriptors.
4. Make `CommandDesc` the **single source of truth** — lift all hardcoded
   command knowledge from the analyser, compiler, and formatter into the
   registry.
5. Enable proc shape inference — the analyser infers how user-defined proc
   parameters flow by querying the registry generically.

## File layout

```
native/include/tcl_lsp/registry/
    command_desc.hpp        — All types: CommandDesc, SubCmdDesc, FormDesc,
                              OptionDesc, ArgPattern, ArgTypeDesc, Arity,
                              DialectFlags, LayoutResolver, HoverText, enums
    command_registry.hpp    — CommandRegistry class, ResolveResult, hover rendering

native/src/registry/
    command_registry.cpp    — find, resolve_arg_roles, layout resolvers, hover
    sample_commands.cpp     — Constexpr descriptors for 17 sample commands
                              covering all 10 shape patterns (test fixtures)
```

## Core types

### ArgPattern — the key innovation

Replaces `dict[int, ArgRole]` with a tagged-union that can express four
patterns:

| Kind | Meaning | Example |
|------|---------|---------|
| `FIXED` | Role at a specific index | `proc`: arg 2 = BODY |
| `TAIL` | From index N onward | `lassign`: args 1+ = VAR_NAME |
| `STRIDE` | Repeating at step S | `foreach`: pairs at stride 2 |
| `OPTION_VALUE` | Value of named option | tcltest `-body` = BODY |

### Arity — with modular constraints

```cpp
struct Arity {
    int32_t min, max;
    int32_t modulus, remainder;  // e.g. foreach: odd argc
};
```

### DialectFlags — bitmask visibility

Commands declare which dialects they belong to.  A lookup expands the
active dialect to its visibility set (e.g. `IRULES` sees `TCL84 | IRULES`).

### LayoutResolver — variable-layout commands

Commands like `if`, `switch`, `try` have keyword-driven argument layouts
that can't be expressed as static patterns.  The `LayoutResolver` enum
dispatches to C++ functions that walk args at runtime.

Currently implemented: `IF`, `SWITCH`, `TRY`, `WHEN`.
Stubbed (fall through to static patterns): `FOREACH`, `EXPECT`, `TCLTEST`,
`OO_CLASS`, `OO_DEFINE`, `OO_DEFINITION`, `OO_PRIVATE`, `OO_SELF`.

### ArgTypeDesc — shimmer detection

Per-argument type expectations for the type inference pass:

```cpp
struct ArgTypeDesc {
    int16_t index;
    TclType expected;
    bool shimmers;  // forces conversion → O130 diagnostic
};
```

### HoverText — 6-field hover data

Matches Python's `HoverSnippet` with `summary`, `synopsis`, `snippet`,
`source`, `examples`, `return_value`.

## The 10 command shapes

| # | Shape | Example | ArgPattern kinds used |
|---|-------|---------|----------------------|
| 1 | Fixed positional | `proc name argList body` | FIXED |
| 2 | Unlimited tail | `lassign list ?varName ...?` | FIXED + TAIL |
| 3 | Stride pattern | `foreach vL list ... body` | STRIDE + FIXED(-1) |
| 4 | Option value roles | `tcltest::test -body {..}` | OptionDesc.value_role |
| 5 | Options + terminator | `regexp -nocase -- pat str` | FIXED + TAIL + OptionDesc |
| 6 | Per-subcmd options | `string compare -nocase` | SubCmdDesc.options |
| 7 | Getter/setter forms | `HTTP::uri` / `HTTP::uri $val` | FormDesc |
| 8 | Variable-layout | `if`/`switch`/`try`/`when` | LayoutResolver |
| 9 | TclOO definitions | `oo::define Class method ...` | LayoutResolver + SubCmdDesc |
| 10 | Simple iRules | `llookup MMAP KEY` | Arity + ArgTypeDesc |

## Resolve algorithm

`CommandRegistry::resolve_arg_roles(cmd, args, expand_flags)`:

1. If a `LayoutResolver` is implemented for the command, dispatch to it.
2. Otherwise, apply static `ArgPattern` entries:
   - `FIXED`: set `roles[index]` (index -1 = last arg)
   - `TAIL`: set `roles[i]` for all `i >= start`
   - `STRIDE`: set `roles[start + k*stride]` for all valid k
3. Apply `OPTION_VALUE` patterns (scan args for option name).
4. Apply `OptionDesc.value_role` for options with semantic roles.
5. When `{*}` expansion flags are set, mark `has_expansion = true`
   (positional roles after expansion become uncertain).

Returns `std::expected<ResolveResult, ResolveError>`.

## Compiler requirements

- **Minimum**: GCC 13 with C++23 (current Ubuntu 24.04 default).
  All types compile and test cleanly.
- **Recommended**: GCC 14+ for reliable `std::expected`.
- **Future**: GCC 15 for `std::flat_map` (drop-in upgrade for sorted vector).

## Migration path

1. **Phase A** (this PR): Core types, registry, sample commands, tests.
2. **Phase B**: Python codegen script to auto-migrate all 1000+ commands.
3. **Phase C**: Wire into C++ analyser, replace `CommandRegistryInterface`.
4. **Phase D**: Implement remaining layout resolvers.
5. **Phase E**: Add hover text, taint hints, side effect hints.

## Verification

- 21 Catch2 test cases with 196 assertions covering all 10 shapes.
- Dialect filtering, arity constraints, modular arity.
- Layout resolvers for if/switch/try/when.
- Hover rendering (markdown and lean).
- `{*}` expansion flag propagation.
- Full test suite (35/35 passing).
