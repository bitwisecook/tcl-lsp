### Optimisation Codes

| Code | Description | Default |
|------|-------------|---------|
| O100 | Propagate constant variables into expressions and command arguments. | ✓ |
| O101 | Fold constant integer expressions. | ✓ |
| O102 | Fold constant `[expr {...}]` command substitutions. | ✓ |
| O103 | Fold static procedure calls using interprocedural summaries. | ✓ |
| O104 | Fold static string build chains into a single assignment. | ✓ |
| O105 | Propagate constants into variable references and detect redundant computations (GVN/CSE). | ✓ |
| O106 | Hoist loop-invariant computations. | ✓ |
| O107 | Eliminate unreachable dead code. | ✓ |
| O108 | Eliminate transitively dead code. | ✓ |
| O109 | Eliminate dead stores. | ✓ |
| O110 | Canonicalise expressions (InstCombine). | ✓ |
| O111 | Brace expression performance hints (paired with W100). | ✓ |
| O112 | Eliminate constant-condition compound statements. | ✓ |
| O113 | Strength-reduce expressions (`x**2` → `x*x`, `x%8` → `x&7`). | ✓ |
| O114 | Recognise `incr` idiom (`set x [expr {$x + N}]` → `incr x N`). | ✓ |
| O115 | Remove redundant nested `[expr {...}]` in expression context. | ✓ |
| O116 | Fold constant `[list a b c]` to literal value. | ✓ |
| O117 | Simplify `[string length $s] == 0` → `$s eq ""`. | ✓ |
| O118 | Fold constant `[lindex {a b c} 1]` to element. | ✓ |
| O119 | Pack consecutive `set` literals into `lassign`/`foreach`. | ✓ |
| O120 | Prefer `eq`/`ne` over `==`/`!=` for string comparisons. | ✓ |
| O121 | Rewrite self-recursive tail calls to `tailcall`. | ✓ |
| O122 | Convert fully tail-recursive proc to iterative `while` loop. | ✓ |
| O123 | Detect non-tail recursion eligible for accumulator introduction (hint only). | ✓ |
| O124 | Comment out unused procs in iRules (not called from any event). | ✓ |
| O125 | Sink side-effect-free assignments into the deepest decision block (`if`/`switch`) that uses them. | ✓ |
| O126 | Remove unused variable assignments — eliminate `set` statements for variables that are never read. | ✓ |
| O127 | Inline single-use variable assignment — eliminate redundant variable load by folding `set` into the use site. | ✓ |
