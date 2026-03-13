# KCS: Authoring Tcl scripts for examples and tests

## Goal

Produce small Tcl scripts that isolate one behaviour so failures are obvious.

## Checklist

- Keep scripts focused on one primary capability.
- Prefer explicit variable names tied to intent (`list_data`, `numeric_total`).
- Avoid unnecessary command substitutions unless that is the thing being tested.
- Include loop versions for performance-sensitive passes (optimiser, shimmer).

## Suggested script categories

1. Parse/lex edge case scripts (quoting, braces, substitutions).
2. IR/CFG/SSA scripts (branches, merges, loops, proc params).
3. Diagnostics scripts (single warning family at a time).
4. Bytecode identity scripts (small snippets with deterministic disassembly).

## Naming guidance

- Use capability prefix + behaviour suffix.
- Examples:
  - `parse_unclosed_brace_nested_cmd.tcl`
  - `ssa_phi_string_int_merge.tcl`
  - `shimmer_loop_list_string_toggle.tcl`

## Storage guidance

- Put reusable fixtures under `tests/fixtures/`.
- Keep exploratory demos under `samples/for_screenshots/`.
