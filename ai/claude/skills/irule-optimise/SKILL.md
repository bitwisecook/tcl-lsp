---
name: irule-optimise
description: >
  Apply LSP optimiser suggestions to an iRule and explain why each
  optimisation is safe and beneficial. Covers constant folding,
  propagation, dead code elimination, and expression canonicalisation.
allowed-tools: Bash, Read, Edit
---

# iRule Optimise

Apply LSP optimiser suggestions with explanations.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Read the iRule file to optimise
3. Run the optimiser:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py optimize $FILE
   ```
4. If no optimisations found, report the code is already well-optimised
5. If the tool outputs an "Optimized Source" section, apply it to the file using the Edit tool
6. For each optimisation applied, explain in 1-2 sentences:
   - Why it is safe (preserves behaviour)
   - What performance or clarity benefit it provides
7. Show a summary of all optimisations applied

## Optimisation codes reference

- O100: Constant propagation — inline known constant values
- O101: Constant folding — evaluate compile-time constant expressions
- O102: Expr folding — fold constant expressions inside expr
- O103: Proc call folding — fold calls to known pure procedures
- O104: String chain folding — combine consecutive string operations
- O105: Constant var-ref propagation / redundant computation (CSE) — propagate constants into $var references and eliminate duplicate expressions
- O106: Loop-invariant code motion — hoist loop-invariant computations
- O107: Dead code elimination — remove unreachable code
- O108: Aggressive dead code elimination — remove unused computations
- O109: Dead store elimination — remove writes to never-read variables
- O110: InstCombine — expression canonicalisation, De Morgan's law, comparison inversion
- O111: Brace expression text for bytecode compilation (paired with W100)
- O112: Structure elimination — remove constant-condition compound statements
- O113: Strength reduction — `x**2` → `x*x`, `x%8` → `x&7`
- O114: Incr idiom — `set x [expr {$x + N}]` → `incr x N`
- O115: Nested expr removal — redundant `[expr {...}]` in expression context
- O116: List folding — `[list a b c]` → literal value
- O117: Strlen simplification — `[string length $s] == 0` → `$s eq ""`
- O118: Lindex folding — `[lindex {a b c} 1]` → element
- O119: Set packing — consecutive `set` literals → `lassign`/`foreach`
- O120: String comparison — prefer `eq`/`ne` over `==`/`!=` for string comparisons
- O121: Tail-call — rewrite self-recursive tail calls to `tailcall`
- O122: Recursion elimination — convert fully tail-recursive proc to iterative loop
- O123: Accumulator hint — detect non-tail recursion eligible for accumulator introduction (advisory only)
- O124: Unused proc commenting — comment out procs not called from any event in iRules
- O125: Code sinking — sink side-effect-free assignments into decision blocks
- O126: Unused variable removal — remove `set` statements for variables never read

## Grouped optimisations

The optimiser automatically groups causally-linked passes. When constant
propagation/folding (O100/O101/O102/O103/O105) makes a variable definition
dead, the resulting dead store elimination (O109) is grouped with the
propagation as one logical optimisation. The tool output shows these as a
single item with sub-entries. When explaining grouped optimisations, treat
them as one transformation (e.g. "propagated constant $x = 5 into the
expression and removed the now-unnecessary `set x 5`").

$ARGUMENTS
