### Diagnostic Codes

| Code | Section | Description | Default |
|------|---------|-------------|---------|
| E001 | error | Missing subcommand — e.g. bare `string` without a subcommand. | ✓ |
| E002 | error | Too few arguments for command. | ✓ |
| E003 | error | Too many arguments for command. | ✓ |
| E200 | error | Shimmer parse error — internal representation cannot be determined. | ✓ |
| W001 | warning | Unknown subcommand. | ✓ |
| W002 | warning | Command is disabled in active dialect profile. | ✓ |
| W100 | warning | Unbraced expression argument — prevents byte-compilation and risks double substitution. | ✓ |
| W104 | warning | String concatenation for list building — use `lappend` instead. | ✓ |
| W105 | warning | Unbraced code block or missing `variable` declaration in `namespace eval`. | ✓ |
| W106 | warning | Dangerous unbraced `switch` body — risks double substitution. | ✓ |
| W108 | warning | Non-ASCII characters in token content. | ✓ |
| W110 | warning | Use `eq`/`ne` instead of `==`/`!=` for string comparison. | ✓ |
| W111 | warning | Line exceeds maximum length (see `tclLsp.style.lineLength`). | ✓ |
| W112 | warning | Trailing whitespace. | ✓ |
| W113 | warning | Procedure shadows built-in command. | ✓ |
| W114 | warning | Redundant nested `[expr {...}]` — already in expression context. | ✓ |
| W115 | warning | Backslash-newline in comment silently swallows the next line. | ✓ |
| W116 | warning | Stub command shadows built-in command. | ✓ |
| W117 | warning | Stub expression definition shadows built-in function or operator. | ✓ |
| W120 | warning | Command used without a corresponding `package require`. | ✓ |
| W121 | warning | Subnet mask has non-contiguous bits. | ✓ |
| W122 | warning | Mistyped IPv4 address (octet > 255 or leading zero). | ✓ |
| W200 | warning | `exec` result not captured or binary format modifier requires newer Tcl. | ✓ |
| W201 | warning | Manual path concatenation — use `file join` instead. | ✓ |
| W210 | variable | Variable read before set. | ✓ |
| W211 | variable | Variable set but never used. | ✓ |
| W212 | variable | Variable substitution where name expected (`set $x`, `incr $x`, `info exists $x`, etc.). | ✓ |
| W213 | variable | Variable may not exist — use `unset -nocomplain` to suppress the error. | ✓ |
| W214 | variable | Unused proc parameter — argument is declared but never read in the procedure body. | ✓ |
| W220 | variable | Dead store — variable set but overwritten before use. | ✓ |
| W101 | security | `eval` with string concatenation — code injection risk. | ✓ |
| W102 | security | `subst` on variable input — code injection risk. | ✓ |
| W103 | security | `open` with pipeline `|` — command injection risk. | ✓ |
| W300 | security | `source` with variable argument — code execution risk. | ✓ |
| W301 | security | `uplevel` with string-built script — injection risk. | ✓ |
| W302 | security | `catch` without result variable — errors are silently swallowed. | ✓ |
| W303 | security | Regexp vulnerable to catastrophic backtracking (ReDoS). | ✓ |
| W304 | security | Missing option terminator `--` on option-bearing commands. | ✓ |
| W306 | security | Substitution in literal-expected argument position. | ✓ |
| W307 | security | Non-literal command name — variable or command substitution as command. | ✓ |
| W308 | security | `subst` without `-nocommands` — risk of unintended command execution. | ✓ |
| W309 | security | `eval`/`uplevel` with `subst` — double substitution risk. | ✓ |
| H300 | hint | Possible paste error — repeated assignment to same variable with same value. | ✓ |
| S100 | shimmer | Single shimmer outside a loop — object internal representation changed. | ✓ |
| S101 | shimmer | Shimmer inside a loop body — per-iteration representation conversion cost. | ✓ |
| S102 | shimmer | Variable oscillates between two types across loop iterations. | ✓ |
| T100 | taint | Tainted data flows into a dangerous code-execution sink (`eval`, `expr`, `exec`, `uplevel`, `subst`). | ✓ |
| T101 | taint | Tainted data flows into an output command (`puts`). | ✓ |
| T102 | taint | Tainted data in option position without `--` terminator — option injection risk. | ✓ |
| IRULE1001 | irules | Command invalid or ineffective in this iRules event. | ✓ |
| IRULE1002 | irules | Unknown iRules event name. | ✓ |
| IRULE1003 | irules | Deprecated iRules event. | ✓ |
| IRULE1004 | irules | `when` block missing explicit `priority`. | ✓ |
| IRULE1005 | irules | Data event without a matching `*::collect` call. | ✓ |
| IRULE1006 | irules | `*::payload` without a matching `*::collect` call. | ✓ |
| IRULE1007 | irules | `*::collect` without a matching `*::release` on the same connection side. | ✓ |
| IRULE1008 | irules | `*::release` without a matching `*::collect` on the same connection side. | ✓ |
| IRULE1201 | irules | HTTP command used after `HTTP::respond`/`HTTP::redirect`. | ✓ |
| IRULE1202 | irules | Multiple `HTTP::respond`/`HTTP::redirect` on different branches. | ✓ |
| IRULE2001 | irules | Deprecated `matchclass` — use `class match` instead. | ✓ |
| IRULE2002 | irules | Deprecated iRules command. | ✓ |
| IRULE2003 | irules | Unsafe iRules command. | ✓ |
| IRULE2101 | irules | Heavy `regexp` in a high-frequency event — consider `string match` or data-group. | ✓ |
| IRULE5001 | irules | Ungated `log` in a high-frequency event. | ✓ |
| IRULE5002 | irules | `drop`/`reject`/`discard` without `event disable all` or `return`. | ✓ |
| IRULE5004 | irules | `DNS::return` without `return`. | ✓ |
| IRULE5005 | irules | Direct proc invocation without `call` — use `call proc_name`. | ✓ |
| IRULE5006 | irules | Top-level-only command used inside a nested body. | ✓ |
| IRULE5007 | irules | Event-context command used at top level outside a `when` block. | ✓ |
| IRULE3001 | irules_security | Tainted data in HTTP response body. | ✓ |
| IRULE3002 | irules_security | Tainted data in HTTP header or cookie value. | ✓ |
| IRULE3003 | irules_security | Tainted data in `log` command — log injection risk. | ✓ |
| IRULE3101 | irules_security | `HTTP::uri`/`HTTP::path` set to value not provably starting with `/`. | ✓ |
| IRULE3102 | irules_security | `HTTP::path`/`HTTP::uri`/`HTTP::query` getter used without `-normalized`. | ✓ |
| IRULE4001 | irules_variable | Write to `static::` variable outside `RULE_INIT`. | ✓ |
| IRULE4002 | irules_variable | Generic `static::` variable name — collision likely across iRules. | ✓ |
| IRULE4003 | irules_variable | Variable scoping concern across events. | ✓ |
| IRULE4004 | irules_variable | Constant `set` in per-request event could be hoisted to an earlier once-per-connection event. | ✓ |
| IRULE4005 | irules_variable | Potential race — `static::` variable written outside `RULE_INIT` and read in another event. | ✓ |

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
