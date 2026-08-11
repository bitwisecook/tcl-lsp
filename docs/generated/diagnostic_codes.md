### Diagnostic Codes

| Code | Section | Description | Default |
|------|---------|-------------|---------|
| E001 | error | Missing dispatch word — e.g. bare `string` without a subcommand, or `$obj` with no TclOO method. | ✓ |
| E002 | error | Too few arguments for command. | ✓ |
| E003 | error | Too many arguments for command. | ✓ |
| E004 | error | Malformed `if` command — missing clauses or extra words after `else`. | ✓ |
| E005 | error | Wrong argument-count shape for command — an in-range count that doesn't fit the command's key/value-pair or paired-argument pattern (e.g. an odd `dict create` tail, an unpaired `foreach` list, or a `switch` count matching neither its shorthand nor its pattern/body-pair form). | ✓ |
| E006 | error | Invalid literal formal-parameter list — Tcl cannot create the procedure or method. | ✓ |
| E100 | error | Unmatched `]` — missing opening `[`? | ✓ |
| E101 | error | Missing `{` after `switch` — case bodies follow without braces. | ✓ |
| E102 | error | Unmatched `}` — missing opening `{`? | ✓ |
| E103 | error | Missing `}` — a nested body consumed this closing brace. | ✓ |
| E200 | error | Unterminated command — the parser could not tell where it ends (missing `]` / `"` / `}`). | ✓ |
| E201 | error | Unterminated command substitution — missing close bracket `]`. | ✓ |
| E202 | error | Unterminated double-quoted string — missing closing `"`. | ✓ |
| E203 | error | Unterminated braced word — missing closing `}`. | ✓ |
| E204 | error | Extra characters after the close brace of a `${name}` variable reference. | ✓ |
| E205 | error | Extra characters after the close quote in a variable name. | ✓ |
| E206 | error | Missing close brace for a `${name}` variable reference. | ✓ |
| E207 | error | Nesting depth exceeds the analysis limit — diagnostics past this point are not collected (matches Tcl's own `interp recursionlimit` error, but reported as a diagnostic rather than a runtime error). | ✓ |
| W001 | warning | Unknown subcommand. | ✓ |
| W002 | warning | Command is disabled in active dialect profile. | ✓ |
| W003 | warning | Expression operator not available in active dialect. | ✓ |
| W004 | warning | Command option is not available in the active dialect. | ✓ |
| W100 | warning | Unbraced expression argument — prevents byte-compilation and risks double substitution. Escalates to Error when the argument provably contains a substitution. | ✓ |
| W104 | warning | String concatenation for list building — use `lappend` instead. | ✓ |
| W105 | warning | Unbraced code block or missing `variable` declaration in `namespace eval`. Escalates to Error when the block provably contains a substitution (double-substitution risk). | ✓ |
| W106 | warning | Dangerous unbraced `switch` body — risks double substitution. | ✓ |
| W107 | warning | Source is not valid UTF-8 — ill-formed bytes were replaced with U+FFFD before analysis, so the analysed text is not the file on disk. | ✓ |
| W108 | warning | Non-ASCII characters in token content. | ✓ |
| W109 | warning | Source does not look like UTF-8 text — it appears to be UTF-16/UTF-32 or binary; the rest of the analysis abstains rather than reporting findings derived from mis-decoded bytes. | ✓ |
| W110 | warning | Use `eq`/`ne` instead of `==`/`!=` for string comparison. | ✓ |
| W111 | warning | Line exceeds maximum length (see `tclLsp.style.lineLength`). | ✓ |
| W112 | warning | Trailing whitespace. | ✓ |
| W113 | warning | Procedure shadows built-in command. | ✓ |
| W114 | warning | Redundant nested `[expr {...}]` — already in expression context. | ✓ |
| W115 | warning | Backslash-newline in comment silently swallows the next line. | ✓ |
| W116 | warning | Stub command shadows built-in command. | ✓ |
| W117 | warning | Stub expression definition shadows built-in function or operator. | ✓ |
| W118 | warning | Inconsistent line endings. | ✓ |
| W120 | warning | Command used without a corresponding `package require`. | ✓ |
| W121 | warning | Subnet mask has non-contiguous bits. | ✓ |
| W124 | warning | Invalid IP address literal. | ✓ |
| W125 | warning | Orphaned control-flow keyword used as standalone command. | ✓ |
| W126 | warning | Non-channel value in channel argument position. | ✓ |
| W127 | warning | Value not in the command's allowed set. | ✓ |
| W128 | warning | Command called after it was renamed or deleted earlier in this file; the call falls through to the `unknown` handler. | ✓ |
| W129 | warning | Command is hidden in a safe interpreter — the call raises `invalid command name` unless it is exposed or reached via `interp invokehidden`. | ✓ |
| W135 | warning | Command requires a newer package version than the resolved `package require`. | ✓ |
| W136 | warning | Option requires a newer package version than the resolved `package require`. | ✓ |
| W137 | warning | Argument value requires a newer Tcl version than the dialect provides. | ✓ |
| W138 | warning | Format/scan conversion requires a newer Tcl version than the dialect provides. | ✓ |
| W139 | warning | Command/option retired at the resolved package version — the retiring release is exclusive, so the item is gone from that release onward. | ✓ |
| W140 | warning | `interp eval` / `interp` subcommand targets an interpreter path never created in this file — the call raises `could not find interpreter` at run time. | ✓ |
| W141 | warning | Option value fails a declared shape/content check (e.g. `-errorstack` must be an even-sized list) — the option-value sibling of W127 for a value that is structurally malformed rather than outside a closed set. | ✓ |
| W142 | warning | Command invalid in its current lexical/dispatch context (e.g. `return` with arguments directly inside an iRules event body). | ✓ |
| W143 | warning | Direct call into a private `::tcl::` implementation namespace (e.g. `::tcl::dict::create`) — use the public ensemble command instead (`dict create`). | ✓ |
| W144 | warning | Command/subcommand/option/argument value is deprecated at the resolved package or Tcl-core version — still available, but the registry records a deprecating release. | ✓ |
| W145 | warning | Ambiguous keyword abbreviation — the prefix matches more than one subcommand or option, which is a runtime error in Tcl. | ✓ |
| W146 | warning | Literal argument violates a registry-declared relationship or member set (for example, a trace operation list contains an operation invalid for its trace type). | ✓ |
| W200 | warning | `exec` result not captured or binary format modifier requires newer Tcl. | ✓ |
| W201 | warning | Manual path concatenation — use `file join` instead. | ✓ |
| W230 | warning | Constant list index out of range — lindex/lrange/lreplace silently return empty or clamp. | ✓ |
| W231 | warning | Constant list index out of range — lset raises a runtime error. | ✓ |
| W232 | warning | Constant string index out of range — string index/range/replace/insert silently return empty or no-op. | ✓ |
| W233 | warning | Division or modulo by a provably-zero divisor — raises 'divide by zero' at runtime. | ✓ |
| W240 | warning | Loop condition is a constant false — body never executes. | ✓ |
| W241 | warning | Loop is provably infinite — constant-true condition with no break/return, zero/wrong-direction counter step. | ✓ |
| W250 | warning | Instantiating an `oo::abstract` class — abstract classes cannot be created directly; use a concrete subclass. | ✓ |
| W308 | warning | Unknown TclOO method — the method is not defined on the receiver's statically-known class or any of its superclasses. | ✓ |
| W314 | warning | Definition has no absolute (fully-qualified) name — an all-colon name or namespace segment (e.g. a proc or namespace named `:`) is reachable only by relative lookup. | ✓ |
| W315 | warning | Class or object definition cannot run — a `deletemethod`/`renamemethod` names a member that does not exist on the side it is scoped to (for `oo::objdefine`, on the object's own table), or renames onto a name already taken, which aborts the whole definition. | ✓ |
| W210 | variable | Variable read before set. | ✓ |
| W211 | variable | Variable set but never used. | ✓ |
| W212 | variable | Variable substitution where name expected (`set $x`, `incr $x`, `info exists $x`, etc.). | ✓ |
| W213 | variable | Variable may not exist — use `unset -nocomplain` to suppress the error. | ✓ |
| W214 | variable | Unused proc parameter — argument is declared but never read in the procedure body. | ✓ |
| W215 | variable | Variable name unreachable via $-substitution (creatable via set/info exists/upvar but no $-form can read it). | ✓ |
| W216 | variable | Broken brace-form array element reference — ``${arr}(x)`` parses as scalar+literal, ``${arr($foo)}`` does not substitute the index. | ✓ |
| W217 | variable | `unset` unsets nothing — every argument is consumed as an option (`-nocomplain` / `--`); prefix a `-`-named variable with `--`. | ✓ |
| W218 | variable | `args` in a non-final parameter position is an ordinary parameter — it only collects the rest as the last formal. | ✓ |
| W220 | variable | Dead store — variable set but overwritten before use. | ✓ |
| W101 | security | `eval` with string concatenation — code injection risk. | ✓ |
| W102 | security | `subst` on variable input — code injection risk. | ✓ |
| W103 | security | `open` with pipeline `|` — command injection risk. | ✓ |
| W300 | security | `source` with variable argument — code execution risk. | ✓ |
| W301 | security | `uplevel` with string-built script — injection risk. | ✓ |
| W302 | security | `catch` without result variable — errors are silently swallowed. | ✓ |
| W303 | security | Regexp vulnerable to catastrophic backtracking (ReDoS). | ✓ |
| W304 | security | Missing option terminator `--` on option-bearing commands. | ✓ |
| W305 | security | Bidirectional formatting control character in source (Trojan Source) — the code renders to a reviewer in a different order from the one it is parsed and executed in. | ✓ |
| W306 | security | Substitution in literal-expected argument position. | ✓ |
| W307 | security | Non-literal command name — variable or command substitution as command. | ✓ |
| W309 | security | `eval`/`uplevel` with `subst` — double substitution risk. | ✓ |
| W310 | security | Hardcoded credential in a password/auth argument — store secrets outside source. | ✓ |
| W311 | security | Channel set to `-encoding binary` with a non-binary `-translation` — may corrupt data or enable encoding-differential attacks. | ✓ |
| W312 | security | `interp eval` with multiple or unbraced script words — concatenated like `eval`, injection risk. | ✓ |
| W313 | security | Destructive file operation with variable path — path-traversal risk. | ✓ |
| H300 | hint | Possible paste error — repeated assignment to same variable with same value. | ✓ |
| I230 | hint | Constant branch condition — the alternate branch is provably unreachable. | ✓ |
| I231 | hint | Constant switch arm condition — the arm is provably unreachable. | ✓ |
| W123 | hint | Unresolved command — not found in registry, user procs, or `unknown` handler. | ✓ |
| W242 | hint | Loop termination cannot be proven — counter not provably modified by the loop body or step. | ✗ |
| S100 | shimmer | Single shimmer outside a loop — object internal representation changed. | ✓ |
| S101 | shimmer | Shimmer inside a loop body — per-iteration representation conversion cost. | ✓ |
| S102 | shimmer | Variable oscillates between two types across loop iterations. | ✓ |
| S103 | shimmer | Mutation of a potentially shared value copies it — Tcl duplicates a shared value before a `lappend`/`lset`/`dict` write. | ✓ |
| S110 | shimmer | Byte-array value coerced to a string by a string operation — binary representation corrupted. | ✓ |
| T100 | taint | Tainted data flows into a dangerous sink: `eval`/`uplevel`/`subst`/unbraced-`expr`/`exec` (code-execution); braced `expr` operands (numeric/type-coercion). | ✓ |
| T101 | taint | Tainted data flows into an output command (`puts`). | ✓ |
| T102 | taint | Tainted data in option position without `--` terminator — option injection risk. | ✓ |
| T103 | taint | Tainted data in a `regexp`/`regsub` pattern — regex-injection or ReDoS risk. | ✓ |
| T104 | taint | Tainted data in a network-address argument (e.g. `socket`) — SSRF risk. | ✓ |
| T105 | taint | Tainted data in a cross-interpreter eval subcommand (`interp eval`/`invokehidden`) — code-execution risk. | ✓ |
| T106 | taint | Already-encoded value passed through a command that re-encodes it — double-encoding. | ✓ |
| TK1001 | tk | Geometry-manager conflict — `pack` and `grid` used on the same parent. | ✓ |
| TK1002 | tk | Widget path references a non-existent parent widget. | ✓ |
| TK1003 | tk | Unknown option for a widget command. | ✓ |
| IRULE1001 | irules | Command invalid or ineffective in this iRules event. | ✓ |
| IRULE1002 | irules | Unknown iRules event name. | ✓ |
| IRULE1003 | irules | Deprecated iRules event. | ✓ |
| IRULE1004 | irules | Explicit event priority required by the registry policy. | ✓ |
| IRULE1005 | irules | Data event without its required registered collection operation. | ✓ |
| IRULE1006 | irules | Payload access without its required registered collection operation. | ✓ |
| IRULE1007 | irules | Collection without its required registered release operation on the same connection side. | ✓ |
| IRULE1008 | irules | `*::release` without a matching `*::collect` on the same connection side. | ✓ |
| IRULE1201 | irules | HTTP command used after `HTTP::respond`/`HTTP::redirect`. | ✓ |
| IRULE1202 | irules | Multiple `HTTP::respond`/`HTTP::redirect` on different branches. | ✓ |
| IRULE2001 | irules | Deprecated `matchclass` — use `class match` instead. | ✓ |
| IRULE2002 | irules | Deprecated iRules command. | ✓ |
| IRULE2003 | irules | Unsafe iRules command. | ✓ |
| IRULE2101 | irules | Heavy `regexp` in a high-frequency event — consider `string match` or data-group. | ✓ |
| IRULE5001 | irules | Ungated `log` in a high-frequency event. | ✓ |
| IRULE5002 | irules | `drop`/`reject`/`discard` without `event disable all` or `return`. | ✓ |
| IRULE5003 | irules | Loop condition `$x != 0` can skip zero when decremented past it — use `$x > 0`. | ✓ |
| IRULE5004 | irules | `DNS::return` without `return`. | ✓ |
| IRULE5005 | irules | Direct proc invocation without `call` — use `call proc_name`. | ✓ |
| IRULE5006 | irules | Top-level-only command used inside a nested body. | ✓ |
| IRULE5007 | irules | Event-context command used at top level outside a `when` block. | ✓ |
| IRULE6001 | irules | `global`/`::`-qualified variable forces CMP compatibility mode, pinning the virtual server to one TMM — use `static::`. | ✓ |
| IRULE3001 | irules_security | Tainted data in HTTP response body. | ✓ |
| IRULE3002 | irules_security | Tainted data in HTTP header or cookie value. | ✓ |
| IRULE3003 | irules_security | Tainted data in `log` command — log injection risk. | ✓ |
| IRULE3004 | irules_security | Tainted data in an `HTTP::redirect` URL — open-redirect risk. | ✓ |
| IRULE3101 | irules_security | `HTTP::uri`/`HTTP::path` set to value not provably starting with `/`. | ✓ |
| IRULE3102 | irules_security | `HTTP::path`/`HTTP::uri`/`HTTP::query` getter used without `-normalized`. | ✓ |
| IRULE3103 | irules_security | Manual split/match of an un-normalised URI getter — parse-differential / traversal risk. | ✓ |
| IRULE4001 | irules_variable | Write to `static::` variable outside `RULE_INIT`. | ✓ |
| IRULE4002 | irules_variable | Generic `static::` variable name — collision likely across iRules. | ✓ |
| IRULE4003 | irules_variable | Variable scoping concern across events. | ✓ |
| IRULE4004 | irules_variable | Constant `set` in per-request event could be hoisted to an earlier once-per-connection event. | ✓ |
| IRULE4005 | irules_variable | Potential race — `static::` variable written outside `RULE_INIT` and read in another event. | ✓ |
| W130 | tclpkg | tclpkg.tcl requires package but it is not in tclpkg.lock — run 'tcl pkg install'. | ✓ |
| W131 | tclpkg | tclpkg.lock is out of sync with tclpkg.tcl — run 'tcl pkg install'. | ✓ |
| W132 | tclpkg | tclpkg.lock integrity mismatch — CAS hash differs from lockfile. | ✓ |
| W133 | tclpkg | tclpkg.tcl directive not permitted in safe mode. | ✓ |
| W134 | tclpkg | Package resolved but no pkgIndex.tcl found — 'package require' will fail at runtime. | ✓ |
