---
name: tcl-optimise
description: "Apply LSP optimiser suggestions to a Tcl file and explain why each optimisation is safe and beneficial. Covers constant folding, propagation, dead code elimination, strength reduction, and expression canonicalisation. Use when optimising Tcl code, improving .tcl file performance, refactoring Tcl scripts for efficiency, or applying language server optimisation suggestions."
allowed-tools: mcp__tcl-lsp__optimize, mcp__tcl-lsp__analyze, Read, Edit
---

# Tcl Optimise

Apply the optimiser's rewrites to a Tcl file and explain each one.

## Steps

1. Read `../_prompts/tcl_system.md`, then the file.
2. Call `mcp__tcl-lsp__optimize` with the contents as `source` (`profile`
   defaults to `full`; `aggressive` iterates to a fixpoint). On a tool error
   (e.g. a parse failure) report it and suggest fixes; on no findings report
   the code is already well-optimised.
3. Apply the returned optimised source with Edit.
4. For each optimisation, one or two sentences: why it preserves behaviour,
   and what it gains. A grouped item (constant propagation whose dead store
   was then eliminated) is one transformation — explain it as one.
5. Re-validate with `mcp__tcl-lsp__analyze` on the rewritten source; revert
   any optimisation that introduced an issue and say why.
6. Summarise what was applied.

## Optimisation codes

Full table: `docs/generated/optimisation_codes.md`. Categories:

- Readability (O111, O114, O115, O117, O120, O128): idiomatic rewrites (incr, eq/ne, bracing)
- Constant folding/propagation (O100, O101, O102, O103, O105, O110, O113, O116, O118, O129): inline and simplify known values
- Pattern recognition (O104, O119, O130): fold string chains, pack consecutive sets
- Dead code (O107, O108, O109, O112, O124, O126): remove unreachable or unused code and stores
- Code motion (O106, O125, O127): hoist, sink, and inline assignments
- Recursion (O121, O122, O123): tail-call rewriting and accumulator hints

Profiles: off; readability (editor default: O111, O114, O115, O117, O120, O128); standard
(readability + constant folding + pattern recognition); full (every pass,
single pass; CLI/AI default); aggressive (every pass, to a fixpoint).

$ARGUMENTS
