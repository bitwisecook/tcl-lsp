# Regular expressions and pattern matching

**Question:** do the engines, the folder and the analyser match `regexp`, `regsub`, `switch`, `string match`, `lsearch` and `glob` the way tclsh does, and how many regular-expression and glob implementations are there? **Method:** 16 fixtures in [`oo-dispatch-probe/regex-patterns/`](handover/oo-dispatch-probe/regex-patterns/) covering ARE syntax (embedded options, `\m`/`\M`/`\y`, back references, bounds, lookahead, classes, preference rules, directors, collating elements), every `regexp`/`regsub` option, match and count variables, `regsub -command`, `switch -regexp/-glob/-exact` with `-matchvar`/`-indexvar`, `string match` edge cases, filesystem `glob` including `{a,b}`, `lsearch` modes, compile errors and their text, and non-BMP text. Each was run by `run.sh` from a fresh temporary directory under tclsh 8.4.20–9.1.0, Jim, the VM and the runtime (8.6 and 9.0); results in `engines.tsv`. The analyser, rename, references, definition and the optimiser were asked through MCP (`analyser-mcp.out`; a cached build that may lag `afe2f2d8`). The code was then read.

**Verdict:** this area is mostly consolidated and mostly right. There is one ARE engine (`tcl-regex`), reached through one `RegexEngine` implementation by both engines, the registry's folder and the LSP, and it agrees with tclsh on nearly every row, including the preference rules and Tcl 9's `regsub -command`. Its one semantic fault is shared by every caller: after the first match of `-all`, and at any `-start`, C Tcl matches the *rest of the string* with no left context, so `\A`, `\m`, `\y` and their kin fire at the restart point; ours keeps the context. Because the folder uses the same engine, the optimiser writes the wrong result into the user's source. Glob matching has two shared matchers that reproduce tclsh 8.6's own split between `string match` and `switch -glob`, plus a UTF-8 guard in `lsearch` that loses 8.6 non-BMP matches in the VM. On the analysis side, `regexp` match variables no longer give false W210s, but `switch -matchvar`/`-indexvar` do, rename and references refuse them, no invalid literal pattern is diagnosed, and W303 scans text instead of using the parser it has.

## Measured

| Case (fixture) | tclsh 8.4–9.1 | VM | runtime | optimiser (O129) |
| --- | --- | --- | --- | --- |
| `regsub -all {\A.} abc X` (r14) | `XXX` | `Xbc` | `Xbc` | folds to `Xbc` |
| `regsub -all {\m.} "ab cd" X` (r14) | `XX XX` | `Xb Xd` | `Xb Xd` | folds to `{Xb Xd}` |
| `regsub -all {\y} "ab cd" \|` (r14) | `\|a\|b \|c\|d` | `\|ab\| \|cd\|` | same | folds to `{\|ab\| \|cd\|}` |
| `regexp -all -inline {\m.} "ab cd"` (r14) | `a b c d` | `a c` | `a c` | not folded |
| `regexp -inline -start 1 {\A.} abc` (r14, r13) | `b` | empty | empty | not folded |
| `regexp -all -inline {\A.\|.\Z} abc` (r13) | `a b c` | `a c` | `a c` | — |
| `^` with `-all`/`-start` (r13, r14) | no match at a restart | right | right | right |
| compile error text for REG_BADRPT on 8.x (r10, r01) | `quantifier operand invalid` | 9.x text `invalid quantifier operand` | same | — |
| `lsearch -glob` with a non-BMP pattern taken from a list element, 8.6 (r15, r16) | `0` | `-1` | `0` | — |

Jim agrees with tclsh on `\A` and `\m` under `-all` (it has no `\y` and differs on `^`), so the behaviour is the C calling convention, not an ARE rule.

Everything else agrees in both engines on both releases: `(?i)`, `\m\M\y\B`, back references (including `\10` and case-insensitive), bounds and the `{256}` limit, lookahead, lookbehind as a compile error, POSIX, collating and equivalence classes, `***=`/`***:`, the embedded flags `e q x n s w p c i`, the mixed-greediness rule (`(a*?)(a+)` gives `a {} a`), zero-length `-all` matches, `-indices` with unmatched groups (`-1 -1`), `-about`, `-line*`, `-expanded`, negative and past-the-end `-start`, `regsub` `&`, `\0`–`\9`, count variable and return forms, `regsub -command` with sub-matches and a variable (9.0), the 8.x "bad option/switch" texts, `switch` with `-matchvar`/`-indexvar` and the `-glob -matchvar` error, every `string match` edge (`[c-a]`, `[]a]`, `a\` against `a\`, unterminated `[`), filesystem `glob` with `{a,b}`, `{,b}`, `?`, escaped brackets, hidden files and Tcl 9's empty result for no match, and `lsearch -glob/-regexp/-not/-nocase`. tclsh 8.6 itself treats a non-BMP character as two units in `string match ?` but one in `switch -glob ?` (r15); both engines reproduce that.

Analysis (`analyser-mcp.out`):

| Case | Report |
| --- | --- |
| `switch -regexp -matchvar mv -indexvar iv` in a procedure and at top level | W210 on `mv` and `iv` (false), once with "did you mean 'm'?" |
| rename, references, definition on a `-matchvar` variable | refused (`null`), empty, empty |
| `regexp … -> k v`, `regexp … all num`, `regsub … out`, `-indices … where` | no W210 (correct); rename of `k` edits both sites |
| literal invalid patterns: `{a{3,2}}`, `{(}`, `{a**}`, `{(?z)}`, `{a{1000}}`, `(?<=a)b` | nothing |
| `regexp -all -inline … v`, `switch -glob -matchvar` | nothing (each is a tclsh error) |
| `{(a+)+$}` | W303; tclsh 9.0 and the VM both match `a^n b` in time independent of n for n = 20, 40, 80 |
| `{\(a\|b\)+}` (escaped, literal parentheses) | W303, false |
| `regsub -command` on 8.6 | W004, correct |

## Census

**ARE engine: one.** `tcl-regex` (`rust/tcl-regex/src`, about 5,900 lines: parser, NFA, executor, Jim's engine in `jim.rs`, source components in `source.rs`). `RegexEngine` has one production implementation, `AreEngine` (`rust/tcl-regex/src/cmd_core.rs:74`). Callers: `tcl_cmd_core::regex` (`regexp`, `regsub`, `lsearch -regexp`, `switch -regexp`), the VM (`rust/tcl-vm/src/cmd_regexp.rs`, `exec/instructions/opcode_dispatch.rs:1408`), the runtime (`runtime/rust/src/cmd_regex.rs`, `native_switch.rs:197`) and its C ABI shim (`regex_capi.rs`), the registry folder (`rust/tcl-registry/src/value_transfer/regex.rs:267`, `:382`; `selection.rs:344`), `variable_output.rs:141`, and the LSP's hover and semantic tokens through `are_components` (`tcl-lsp-core/src/pattern_components.rs`, `semantic_tokens/regex_subtokens.rs`). The Rust `regex` crate appears only in BIG-IP configuration, policy and tooling code, never for Tcl patterns.

**The restart fault** is in the engine contract: `RegexEngine::exec` takes the whole subject and an offset and says a "context-aware crate engine can ignore" `notbol` (`rust/tcl-cmd-core/src/regex.rs:321–331`), while C's `Tcl_RegExpExecObj` passes the substring from the offset, so only `^` is suppressed (`notbol_at`, `regex.rs:780`) and every other zero-width assertion sees no left context. The registry's `regexp` documentation already says `\A` "still anchors to index" (`commands/tcl/regexp_.rs:270`).

**Messages.** The release prefix ("couldn't"/"cannot compile") is owned once (`rust/tcl-dialect/src/version/release_behaviour.rs:73`), but the REG_* texts are 9.x-only in two tables (`rust/tcl-regex/src/defs.rs:217`, `rust/tcl-cmd-core/src/regex.rs:412`) and hard-coded once more in the runtime (`runtime/rust/src/cmd_switch.rs:505`).

**Glob matchers: two shared, three private.**

- `tcl_syntax::glob::string_case_match` (`rust/tcl-syntax/src/glob.rs:109`), over `char`s: `string match` (cmd-core `string.rs:1419`, VM `cmd_string.rs:246`), `lsearch -glob` (`lsearch.rs:839`), `case` (`case.rs:81`), dict and array filters, LSP import wildcards. Its byte entry falls back to byte equality when either side is not UTF-8 (`glob.rs:36`), and `lsearch` returns false in that case (`lsearch.rs:833–841`); in 8.6 a non-BMP list element is held as a surrogate pair, which is the r16 miss (*read*; the measurement shows the VM only).
- `tcl_syntax::native_glob::c_match` (`rust/tcl-syntax/src/native_glob.rs:712`), over C units by release, plus `jim_match` (`:466`): `switch -glob` (`switch.rs:432`), `info`, `array names`, `namespace` patterns.
- Private: `glob_matches` in `rust/tcl-cli/src/commands/spec.rs:1087` (tag filter, `*` and `?` only, not Tcl), `pattern_matches` over `NamePiece`s in `rust/tcl-compiler/src/dynamic_names.rs:636` (analysis of dynamic names), `glob_to_re` (`glob.rs:291`) for the F5 XC translator.
- `re_to_glob` (`glob.rs:357`), C's `TclReToGlob`, has four callers: `tcl-registry/src/native_regexp_compilation.rs:301`, `native_compilation/string_grammars.rs:419`, `tcl-codegen/src/codegen/cmd_subst/inline_string.rs:474`, `runtime/rust/src/cmd_regex.rs:212`.

**Filesystem `glob`: one owner.** `tcl_cmd_core::filesystem_glob::c_unix_matches` (`rust/tcl-cmd-core/src/filesystem_glob.rs:226`) for both engines; the runtime keeps its own walker (`runtime/rust/src/cmd_fs.rs:1244`, `:1319`) for Jim and host-less runs.

**Analysis.** `regexp`/`regsub` variable writes come from the registry's effect data (the earlier false W210 on match variables is gone). `switch -matchvar`/`-indexvar` writes are modelled in the registry (`value_transfer/selection.rs:588`), but lowering abandons any `switch` with an argument-taking option (`rust/tcl-compiler/src/lowering/structured.rs:204`, `:242`), and the consumer that reports W210 then sees reads in the bodies without the write. There is no diagnostic code for an invalid pattern, although `tcl-regex` compiles every literal pattern for hover and tokens. W303 now reads components from the selected regexp compiler. It reports nested repetition or repeated alternatives in a pattern with a backreference. The timing fixture covers Tcl 8.6.18 and 9.0.4. [Timing evidence](evidence/regex-backreference-timings.md)

## Design

- **Engine contract (L1 grammar, value kernel).** `RegexEngine::exec` takes the subject *from* the offset and reports offsets relative to it, as C does, so restart semantics are a property of the contract, not of each engine. The folder, `lsearch`, `switch` and the LSP inherit the fix because they share the engine. Release-dependent message texts move into the engine's error type keyed by release.
- **Glob (L1).** Keep two matchers because tclsh has two (`Tcl_StringCaseMatch` over characters, the C-unit matcher for names), name them after their C owners, and make both take the value's units rather than requiring UTF-8, so no caller needs a "not UTF-8, so no match" fallback. The tooling matchers stay private and are documented as not Tcl.
- **Registry descriptors.** `-matchvar`, `-indexvar`, `regexp` match variables, `regsub`'s variable and count, and `-failindex` are variable-write effects on the annotated command (decision 4), read by lowering instead of lowering bailing; the write happens before the selected body. Pattern arguments carry a value grammar (ARE under the command's flags, glob by matcher), so diagnostics, hover, tokens and the folder read one parse.
- **Answers.** A literal pattern that fails to compile is a proven error (a new diagnostic over the shared parse); a dynamic pattern is unknown and abstains. W303 becomes a rule over the parse (nested unbounded quantifiers over overlapping alternatives, back references inside repetition) with its claim restated for Tcl's hybrid engine, or is retired if no exponential case can be shown on tclsh.
- **Folding.** O129 over `regexp`/`regsub` stays on the shared engine; with the restart fix landed it folds `-all` and `-start` over every zero-width assertion, declining only a `-start` value the releases read differently.
- **Conformance.** These fixtures become vector rows with columns for every tclsh release, Jim, both engines, the folder, and each LSP consumer of the pattern parse.

## Plan

- Change `RegexEngine::exec` (`rust/tcl-cmd-core/src/regex.rs:321`) and `AreEngine` (`rust/tcl-regex/src/cmd_core.rs:74`) to match the subject from the offset with no left context, keeping `notbol` for `^`; done when r13 and r14 match tclsh 8.4–9.1 in both engines and the folder.
- Until then, make O129 decline `regexp`/`regsub` with `-all` or `-start` when the pattern contains `\A`, `\m`, `\M`, `\y`, `\Y` or `\B` (`rust/tcl-registry/src/value_transfer/regex.rs`); done when the three wrong folds in `analyser-mcp.out` are gone.
- Make REG_* error texts release-keyed in `rust/tcl-regex/src/defs.rs:217` and delete the copies in `rust/tcl-cmd-core/src/regex.rs:412` and `runtime/rust/src/cmd_switch.rs:505`; done when r10 matches tclsh 8.6 in both engines.
- Remove the UTF-8 requirement from `lsearch -glob` (`rust/tcl-cmd-core/src/lsearch.rs:833–841`) and `string_match_bytes` (`rust/tcl-syntax/src/glob.rs:36`) by matching the value's own units; done when r16 matches on VM 8.6.
- Lower `switch` with `-matchvar`/`-indexvar` as a write of those variables before each selected body, from the registry's selection effects (`rust/tcl-compiler/src/lowering/structured.rs:204`, `rust/tcl-registry/src/value_transfer/selection.rs:588`); done when r06 and r11 give no W210 and rename, references and definition work on the variables.
- Add an invalid-pattern diagnostic over the shared `tcl-regex` parse for literal `regexp`, `regsub`, `lsearch -regexp` and `switch -regexp` patterns, and argument-shape errors for `-inline` with match variables and `-matchvar` without `-regexp`; done when every literal error case in `analyser-mcp.out` is reported and no dynamic pattern is.
- Rewrite W303 over the `tcl-regex` parse (`rust/tcl-compiler/src/analyser/diagnostics/security.rs:580`) and measure its claim on tclsh 8.6 and 9.0 with a timing fixture; done when `{\(a|b\)+}` is clean and every remaining W303 shape has a measured slow case, or W303 is retired.
- Name and document the two glob matchers after `Tcl_StringCaseMatch` and the C-unit name matcher, and mark `tcl-cli` `glob_matches` and `dynamic_names::pattern_matches` as non-Tcl; done when the walker manifest lists two Tcl glob owners.
- Turn the regex and glob fixtures into conformance vectors run against tclsh, Jim, both engines, the folder and the LSP's hover and token parse.

## Status on the current source

Item 153 makes the folder decline the restart-sensitive forms in `r14`.
Item 154's engine correction has landed: `AreEngine` matches the subject
from the offset under the caller's `REG_NOTBOL`, and `r14` matches tclsh
8.4–9.1 in the VM and the runtime. It was found through tcllib 2.0's
`string::token::shell`, whose lexer matches `\A(…)` at successive offsets.
Past the end, `-indices` reports the caller's offset as C does
(`regexp -start 10 -indices -inline {\A} abc` is `{10 9}`). The folder no
longer declines the restart forms and folds `r14`'s `regsub -all {\A.} abc X`
to `XXX`. Tcl 8.4's `regexp` (not `regsub`) keeps `REG_NOTBOL` for the whole
call once the offset is positive, so `^` never matches at a restart there,
even after a newline; `TclVersion::regexp_restart_after_newline_is_bol`
holds that fact. W210 on `switch -matchvar` in procedures (`r11`) remains open. [Execution audit](handover/oo-dispatch-probe/tcl-areas-head/README.md)
