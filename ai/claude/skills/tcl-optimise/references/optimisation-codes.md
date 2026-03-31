# Optimisation Codes Reference

Complete list of optimisation codes emitted by the tcl-lsp optimiser.

| Code | Name | Description |
|------|------|-------------|
| O100 | Constant propagation | Inline known constant values |
| O101 | Constant folding | Evaluate compile-time constant expressions |
| O102 | Expr folding | Fold constant expressions inside expr |
| O103 | Proc call folding | Fold calls to known pure procedures |
| O104 | String chain folding | Combine consecutive string operations |
| O105 | Redundant computation / CSE | Eliminate duplicate expressions |
| O106 | Loop-invariant code motion | Hoist loop-invariant computations |
| O107 | Dead code elimination | Remove unreachable code |
| O108 | Aggressive dead code elimination | Remove unused computations |
| O109 | Dead store elimination | Remove writes to never-read variables |
| O110 | InstCombine | Expression canonicalisation, De Morgan's law, comparison inversion |
| O111 | Brace expression text | Bytecode compilation (paired with W100) |
| O112 | Structure elimination | Remove constant-condition compound statements |
| O113 | Strength reduction | `x**2` → `x*x`, `x%8` → `x&7` |
| O114 | Incr idiom | `set x [expr {$x + N}]` → `incr x N` |
| O115 | Nested expr removal | Redundant `[expr {...}]` in expression context |
| O116 | List folding | `[list a b c]` → literal value |
| O117 | Strlen simplification | `[string length $s] == 0` → `$s eq ""` |
| O118 | Lindex folding | `[lindex {a b c} 1]` → element |
| O119 | Set packing | Consecutive `set` literals → `lassign`/`foreach` |
| O120 | String comparison | Prefer `eq`/`ne` over `==`/`!=` for string comparisons |
| O121 | Tail-call | Rewrite self-recursive tail calls to `tailcall` |
| O122 | Recursion elimination | Convert fully tail-recursive proc to iterative loop |
| O123 | Accumulator hint | Detect non-tail recursion eligible for accumulator introduction (advisory only) |
| O124 | Unused proc commenting | Comment out procs not called from any event in iRules (iRules dialect only) |
| O125 | Code sinking | Sink side-effect-free assignments into decision blocks |
| O126 | Unused variable removal | Remove `set` statements for variables never read |
| O127 | Inline single-use assignment | Fold `set` into use site to eliminate redundant variable load |
