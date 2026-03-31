---
name: tcl-optimise
description: "Apply LSP optimiser suggestions to a Tcl file and explain why each optimisation is safe and beneficial. Covers constant folding, propagation, dead code elimination, strength reduction, and expression canonicalisation. Use when optimising Tcl code, improving .tcl file performance, refactoring Tcl scripts for efficiency, or applying language server optimisation suggestions."
allowed-tools: Bash, Read, Edit
---

# Tcl Optimise

Apply LSP optimiser suggestions to Tcl files with safety explanations for each transformation.

## Steps

1. Read the domain knowledge from `ai/prompts/tcl_system.md`
2. Read the Tcl file to optimise
3. Run the optimiser:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py optimize $FILE
   ```
4. If the tool fails (e.g. file not found or parse error), report the error clearly and suggest fixes
5. If no optimisations found, report the code is already well-optimised
6. If the tool outputs an "Optimized Source" section, apply it to the file using the Edit tool
7. For each optimisation applied, explain in 1-2 sentences:
   - Why it is safe (preserves behaviour)
   - What benefit it provides
8. Validate the optimised file to confirm no regressions:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
9. If validation finds new issues, revert the problematic optimisation and explain why

## Optimisation codes reference

See `docs/generated/optimisation_codes.md` for the full auto-generated table of O100+ codes.
Key categories:

- **Constant folding/propagation** (O100-O105): Inline and simplify known values
- **Dead code** (O107-O109, O126): Remove unreachable or unused code and stores
- **Expression** (O110-O115): Canonicalise, brace, and simplify expressions
- **Strength reduction** (O113, O117-O118): Replace expensive ops with cheaper equivalents
- **Restructuring** (O112, O119, O121-O122, O125, O127): Simplify control flow and variable usage

## Grouped optimisations

The optimiser automatically groups causally-linked passes. When constant
propagation/folding makes a variable definition dead, the resulting dead store
elimination is grouped with the propagation as one logical optimisation. The
tool output shows these as a single item with sub-entries. When explaining
grouped optimisations, treat them as one transformation.

$ARGUMENTS
