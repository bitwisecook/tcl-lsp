### Optimisation Codes

| Code | Category | Description | readability | standard | full |
|------|----------|-------------|:-----------:|:--------:|:----:|
| O100 | constant_folding | Propagate constant variables into expressions and command arguments. |  | ✓ | ✓ |
| O101 | constant_folding | Fold constant integer expressions. |  | ✓ | ✓ |
| O102 | constant_folding | Forward a variable's single reaching literal load to its use sites. |  | ✓ | ✓ |
| O103 | constant_folding | Fold static procedure calls using interprocedural summaries. |  | ✓ | ✓ |
| O104 | pattern | Fold static string build chains into a single assignment. |  | ✓ | ✓ |
| O105 | constant_folding | Propagate constants into variable references and detect redundant computations (GVN/CSE). |  | ✓ | ✓ |
| O106 | code_motion | Hoist loop-invariant computations. |  |  | ✓ |
| O107 | dce | Eliminate unreachable dead code. |  |  | ✓ |
| O108 | dce | Eliminate transitively dead code. |  |  | ✓ |
| O109 | dce | Eliminate dead stores. |  |  | ✓ |
| O110 | constant_folding | Canonicalise expressions (InstCombine). |  | ✓ | ✓ |
| O111 | readability | Brace expression performance hints (paired with W100). | ✓ | ✓ | ✓ |
| O112 | dce | Eliminate constant-condition compound statements. |  |  | ✓ |
| O113 | constant_folding | Strength-reduce expressions (`x**2` → `x*x`, `x%8` → `x&7`). |  | ✓ | ✓ |
| O114 | readability | Recognise `incr` idiom (`set x [expr {$x + N}]` → `incr x N`). | ✓ | ✓ | ✓ |
| O115 | readability | Remove redundant nested `[expr {...}]` in expression context. | ✓ | ✓ | ✓ |
| O116 | constant_folding | Fold constant `[list a b c]` to literal value. |  | ✓ | ✓ |
| O117 | readability | Simplify `[string length $s] == 0` → `$s eq ""`. | ✓ | ✓ | ✓ |
| O118 | constant_folding | Fold constant `[lindex {a b c} 1]` to element. |  | ✓ | ✓ |
| O119 | pattern | Pack consecutive `set` literals into `lassign`/`foreach`. |  | ✓ | ✓ |
| O120 | readability | Prefer `eq`/`ne` over `==`/`!=` for string comparisons. | ✓ | ✓ | ✓ |
| O121 | recursion | Rewrite self-recursive tail calls to `tailcall`. |  |  | ✓ |
| O122 | recursion | Convert fully tail-recursive proc to iterative `while` loop. |  |  | ✓ |
| O123 | recursion | Detect non-tail recursion eligible for accumulator introduction (hint only). |  |  | ✓ |
| O124 | dce | Comment out unused procs in iRules (not called from any event). |  |  | ✓ |
| O125 | code_motion | Sink side-effect-free assignments into the deepest decision block (`if`/`switch`) that uses them. |  |  | ✓ |
| O126 | dce | Remove unused variable assignments — eliminate `set` statements for variables that are never read. |  |  | ✓ |
| O127 | code_motion | Inline single-use variable assignment — eliminate redundant variable load by folding `set` into the use site. |  |  | ✓ |
| O128 | readability | Rewrite `[expr {[llength $L] - N}]` / `[expr {[string length $s] - N}]` to `end-(N-1)` when used as an index argument. | ✓ | ✓ | ✓ |
| O129 | constant_folding | Fold a pure builtin command substitution with constant arguments (`[string length ...]`, `[join ...]`, `[format ...]`, `[dict get ...]`, …). |  | ✓ | ✓ |
| O130 | pattern | Fold static `lappend` list build chains into a single assignment. |  | ✓ | ✓ |

**Profiles:** `off` disables all passes. `readability`, `standard`, and `full` enable
progressively more passes (single-pass). `aggressive` = `full` with multi-pass
to fixpoint (up to 5 iterations). The default editor profile is `readability`;
explicit actions (CLI, chat, MCP) default to `full`.
