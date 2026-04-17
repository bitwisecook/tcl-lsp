---
name: irule-optimise
description: "Apply LSP optimiser suggestions to an F5 iRule and explain why each optimisation is safe and beneficial. Covers constant folding, propagation, dead code elimination, strength reduction, and expression canonicalisation. Use when optimising iRule code, improving iRule performance, applying F5 iRule optimisations, or refactoring iRules for efficiency."
allowed-tools: Bash, Read, Edit
---

# iRule Optimise

Apply LSP optimiser suggestions to iRule files with safety explanations for each transformation.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule file to optimise
3. Run the optimiser:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py optimize $FILE
   ```
4. If the tool fails (e.g. file not found or parse error), report the error clearly and suggest fixes
5. If no optimisations found, report the code is already well-optimised
6. If the tool outputs an "Optimized Source" section, apply it to the file using the Edit tool
7. For each optimisation applied, explain in 1-2 sentences:
   - Why it is safe (preserves behaviour)
   - What performance or clarity benefit it provides
8. Show a summary of all optimisations applied
9. Validate the optimised file to confirm no regressions:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
10. If validation finds new issues, revert the problematic optimisation and explain why

## Optimisation codes reference

See `docs/generated/optimisation_codes.md` for the full auto-generated table of O100+ codes.
Key categories:

- **Readability** (O111, O114, O115, O117, O120, O128): Idiomatic rewrites (incr, eq/ne, bracing)
- **Constant folding/propagation** (O100, O101, O102, O103, O105, O110, O113, O116, O118): Inline and simplify known values
- **Pattern recognition** (O104, O119): Fold string chains, pack consecutive sets
- **Dead code** (O107, O108, O109, O112, O124, O126): Remove unreachable or unused code and stores
- **Code motion** (O106, O125, O127): Hoist, sink, and inline assignments
- **Recursion** (O121, O122, O123): Tail-call rewriting and accumulator hints

## Optimiser profiles

- **off**: All optimisations disabled
- **readability** (editor default): O111, O114, O115, O117, O120, O128
- **standard**: readability + constant folding + pattern recognition
- **full** (CLI/AI default): All 28 passes, single pass
- **aggressive**: All passes, multi-pass to fixpoint

## Grouped optimisations

The optimiser automatically groups causally-linked passes. When constant
propagation/folding (O100/O101/O102/O103/O105) makes a variable definition
dead, the resulting dead store elimination (O109) is grouped with the
propagation as one logical optimisation. The tool output shows these as a
single item with sub-entries. When explaining grouped optimisations, treat
them as one transformation (e.g. "propagated constant $x = 5 into the
expression and removed the now-unnecessary `set x 5`").

$ARGUMENTS
