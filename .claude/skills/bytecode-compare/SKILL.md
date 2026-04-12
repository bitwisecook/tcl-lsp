---
name: bytecode-compare
description: >
  Compare our compiler's bytecode output against tclsh reference disassembly
  (8.5, 8.6, 9.0). Use when verifying bytecode correctness, investigating
  instruction sequence differences, or checking that our codegen matches C Tcl.
allowed-tools: Bash, Read
---

# Bytecode Comparison Skill

Compares the bytecode produced by our compiler pipeline (lexer → lowering →
IR → CFG → codegen) against tclsh reference disassembly (8.5, 8.6, 9.0)
for a set of 30 test snippets.

## Usage

Run from the project root:

```bash
python3 .claude/skills/bytecode-compare/bytecode_compare.py [-v VERSION] <subcommand> [args...]
```

## Options

| Flag | Description |
|---|---|
| `-v`, `--version` | Tcl version to compare against: `8.5`, `8.6`, `9.0` (default: `9.0`) |

## Subcommands

| Subcommand | Arguments | What it does |
|---|---|---|
| `all` | | Compare all 30 snippets, show summary table |
| `diff` | `<snippet>` | Detailed instruction-by-instruction diff for one snippet (e.g. `08_while`) |
| `summary` | | One-line per snippet: match/mismatch with instruction counts |
| `instructions` | `<snippet>` | Side-by-side normalised instruction listing |
| `refresh` | | Regenerate our bytecode reference and re-compare |
| `categories` | | Group differences by category (variable access, jumps, opcodes, etc.) |
| `versions` | `<snippet>` | Compare one snippet against all Tcl versions (8.5, 8.6, 9.0) |

## Interpreting Output

### Summary mode
```
 01_set_simple        MISMATCH  ours: 3 instrs  9.0: 4 instrs
 03_expr_braced       MATCH     ours: 2 instrs  9.0: 2 instrs
 22_puts_invoke       MATCH     ours: 4 instrs  9.0: 4 instrs
```
MATCH means identical normalised instruction sequences. MISMATCH means differences exist.

### Diff mode
```
=== 08_while ===
  9.0: (0) push1 0         # "i"
  9.0: (2) push1 1         # "0"
  9.0: (4) storeStk
  ours: (0) push1 0        # "0"
  ours: (2) storeScalar1 %v0  # var "i"
  --- variable access: storeStk (name-based) vs storeScalar1 (LVT-indexed)
```

### Categories mode
Groups all divergences into categories:
- **Variable access**: storeStk/loadStk vs storeScalar1/loadScalar1
- **Jump encoding**: relative offsets vs absolute PC
- **Missing opcodes**: opcodes tclsh uses that we don't emit
- **Extra instructions**: instructions we emit that tclsh doesn't
- **Dead code**: unreachable code differences

## When to use

- After modifying `server/compiler/codegen.py` — verify instruction sequences
- After modifying lowering or CFG — check for regressions
- When investigating why a specific snippet diverges from tclsh
- Before committing codegen changes — run `summary` as a gate check

$ARGUMENTS
