# Comments, continuations and the parsing edge cases

How `#` comments, backslash-newline continuations and `;` are read across the engines, the lexer, the analyser and the editor features (formatter, minifier, folding, semantic tokens, docstrings, style lints), and whether they agree with tclsh. The 27 fixtures in [`oo-dispatch-probe/comments-continuations/`](handover/oo-dispatch-probe/comments-continuations/) each isolate one rule:

- `#` only at command position, after `;` and mid-command;
- the `switch` body trap with odd and even word counts;
- unbalanced and balanced braces in comments, and a quoted brace in a braced body;
- a comment ending in one backslash or in an even run of backslashes;
- backslash-newline in braces, in quotes and bare;
- `;` in quotes and braces;
- `#` after an `if` body, which is the implicit else;
- comments in `if`, `proc` and `namespace eval` bodies;
- `{*}` over a word holding `#`;
- `#` lines in `array set`, `dict create` and `foreach` data;
- `#` in `expr`;
- backslash-space against backslash-newline;
- CRLF continuation;
- a doc comment that swallows the next line.

They were run under tclsh 8.4, 8.6, 9.0 and 9.1, jimsh 0.84, the VM and the runtime (8.6 and 9.0) (`engines.tsv`). The editor features were driven through `target/release/tcl-lsp-server` (built 2026-11-22) over JSON-RPC: diagnostics, semantic tokens, folding ranges, formatting and `tcl-lsp.minifyDocument` at the default and compact tiers. Each formatted and each minified file was run under tclsh 9.0 and its output compared with the original's (`lsp-features.out`). `mcp__tcl-lsp__analyze` was asked as well (`analyser-mcp.out`).

**Verdict:** the parse is right almost everywhere. Both engines and every analysis path segment through `tcl_lexer`, whose comment and continuation rules match tclsh. All 27 formatted and all 27 minified outputs behave as the originals. The faults lie at the edges:

- The runtime does not translate CRLF when it reads a script, so a CRLF backslash-newline is not a continuation there (a wrong answer, also through `source`).
- W115 fires on a comment ending in an even run of backslashes, which does not continue.
- The analyser says nothing about a `switch` body whose `#` line makes the word count odd, a certain runtime error on every release.
- `#` in `expr` under 8.6 goes unreported.
- No lint says that a `#` line in a data list (`switch`, `array set`, `dict create`, `foreach`) is data.

Outside the lexer there are four text-level re-implementations of "is this a comment", two of which disagree with it.

## Measured

**Engines** (tclsh 8.6 as reference; every fixture agrees across 8.6, 9.0 and 9.1 except c18):

| Case | tclsh | Jim | VM | Runtime |
| --- | --- | --- | --- | --- |
| `set a 1\` CRLF `puts …` (c21) | continuation (file read translates CRLF) | escaped CR, no continuation | as tclsh | **as Jim: no continuation**, also through `source` |
| backslash-newline in braces (c08) | folds to one space (length 3) | kept literally (length 10) | as tclsh | as tclsh |
| `{*}{a # b …}`, `dict create {*}{# x …}` (c14, c15) | 8.5+: `#` is an element | as 8.6 | as tclsh | as tclsh |
| `#` in `expr` (c18) | error in 8.4–8.6, accepted in 9.0+ | accepted | per release | per release |
| odd-length `switch` body (c03) | "extra switch pattern with no body, this may be due to a comment …" | usage error | as tclsh | as tclsh |
| error location (c20) | `(file … line 1)` with "invoked from within" | | `line 2`, no "invoked from within" | message only |

**Analysis and editor features** (any diagnostic on a correct program is false; "none" on a failing one is a miss):

| Case | Report |
| --- | --- |
| odd-length `switch` body from a `#` line (c03) | **none** (MCP and server) |
| six-word `#` line pairing into patterns and bodies (c26) | W123 on `this`, `not`, `comment`; this matches what tclsh runs, but says nothing about the comment |
| comment ending in `\\` (even run) (c27) | **W115 "swallows the next line" (false)**; all engines print the next line |
| comment ending in `\` (c07, c19, c25) | W115 from the server; none from `mcp__tcl-lsp__analyze`, which omits the source-style pass |
| `#` in `expr` under `tcl8.6` (c18, t20) | **none** |
| `#` line in `array set`/`dict create`/`foreach` data (c15, c24) | none |
| `if 0 {…} # comment` (c12) | E004, correct |
| `[list x;y]` (c11), `list a\ ` then newline (c20) | W123 on `y`, `b`, `d` suggesting "did you mean '!'?" |
| `puts "{"` in a braced body (c16) | E200 "missing close-bracket"; tclsh says "missing close-brace" |
| doc comment ending in `\` (c25) | hover on the next proc shows the swallowed `proc hidden {} {}` line as doc text |
| folding | switch body folds when even-length (c26), not when odd (c03) |
| formatter, minifier | output equivalent to the original in all 27; the minifier keeps `#` lines inside `switch`/data lists, strips real comments including continued ones, and refuses malformed sources with `incomplete-original-syntax` |
| semantic tokens | `#` words in data and `switch` bodies are classed as words, continued comment lines as comment |

## Census

**One owner.** `tcl_lexer` reads comments and continuations for both engines (`runtime/rust/src/parse.rs:73` uses `group_commands_bytes`; 27 VM files use `tcl_lexer`) and for analysis. The release and dialect differences are grammar data in `tcl-dialect/src/grammar.rs`: `BraceBackslashNewline` (:983, Jim keeps it literally) and `ExprCommentStyle` (:244). Inside the lexer, the comment end is computed five times with the same rule, consuming one canonical escape at a time:

- `lexer.rs:593` (`parse_comment`);
- `ranges.rs:920` (`comment_end`);
- `structural_index.rs:340` and `:807` (`scan_comment_inert`);
- `structural_index.rs:1218` (`skip_comment`, whose comment records a known EOF difference from `Tcl_CommandComplete`).

`comment_line_starts` (`lib.rs:167`) and `script.rs:230` (`comment_text`, the docstring source) sit on top of the token stream.

**Outside the lexer**, the comment rule is re-derived from text in four places:

- `tcl-lsp-core/src/source_style.rs:406` (`comment_continuation_run`) and `:422` (`…_with_facts`) decide W115 and its code action (`code_actions.rs:837`) with `ends_with('\\')`. That is wrong for an even run (c27). The `_with_facts` variant takes lexer-confirmed comment lines but repeats the same text test for the continuation.
- `tcl-lsp-core/src/inert_text.rs:89` (`offset_in_comment`) is a line-local scanner with its own quote and brace tracking and a `continues_previous_line` helper.
- `tcl-compiler/src/analyser/commands/nested_invocations.rs:925` (`first_command_head`) skips `#` to the newline and ignores backslash-newline, so a continued comment's next line is read as a command head (*read*; no fixture reaches it).
- `tcl-compiler/src/analyser/utils/script_comments.rs:80` (`script_comment_facts`) is lexer-based and is the right model.

**Also re-derived:**

- The `switch` comment hint: `tcl-cmd-core/src/switch.rs:673` builds the message, but each engine decides "has a comment" itself (`tcl-vm/src/cmd_switch.rs:73`, `runtime/rust/src/cmd_switch.rs:187`). The minifier knows the odd-length case (`tcl-lsp-core/src/minify/case_lists.rs:154`); no diagnostic does.
- Formatting has its own comment handling (`formatting/engine.rs:267`, `:486`, `engine/command_parsing.rs:36`, `engine/line_wrapping.rs:233`), and docstrings have theirs (`formatting/docstring.rs:162`). Both behaved correctly here.

**Source reading:** the runtime's script reader passes raw bytes (`runtime/rust/examples/run_script.rs:174`, and its `source`), with no end-of-line translation. The VM translates. That is two readers with different EOL rules (the runtime's `source` path is inferred from c21's behaviour through `source`).

## Design

Comments and continuations belong to grammars (L1) and the source model (L3) of the [architecture plan](architecture-tidy-plan.md). The lexer already owns them, and the structured word should carry them:

- a comment is a node of the script tree, with its continued extent;
- a backslash-newline is a span whose meaning (fold to a space, or keep it, as in Jim's braces) comes from `LexerGrammar`.

No consumer should test `#` or `\` on text. W115, its code action, `offset_in_comment`, docstrings and `first_command_head` should ask the source model: "is this offset in a comment?", "where does this comment end?", "which lines does it swallow?". This is fault 7 of the [cross-cutting review](cross-cutting-review.md): re-reading text beside an owner.

The `switch` trap and the `#`-in-data lint need the registry to declare data-list arguments that are pattern/body lists or key/value lists, as value grammars (fault 9). The checker then counts the words of the parsed list, and the engines' "has a comment" hint becomes one function in `tcl-cmd-core` over the same list.

Reading a script file belongs to the engine traits: one reader that applies the release's `source` translation and encoding, shared by both engines and by `source`. Conformance (fault 5): the 27 fixtures become matrix rows with columns for tclsh, Jim, both engines, the analyser codes, the formatter's and minifier's behavioural equivalence, and folding.

## Plan

- **Runtime CRLF.** Make the runtime's script reader and `source` apply the `auto` end-of-line translation tclsh applies, through one reader shared with the VM behind the engine traits (`runtime/rust`, `tcl-runtime-api`). Done when c21 prints tclsh's error in the runtime, as an end-to-end test.
- **W115 from the lexer.** Replace `ends_with('\\')` in `comment_continuation_run` and `comment_continuation_run_with_facts` (`tcl-lsp-core/src/source_style.rs:406`, `:422`) with the lexer's comment extent (`comment_line_starts` / `script_comment_facts`), and check the code action (`code_actions.rs:837`) against it. Done when c27 has no W115, c07, c19 and c25 keep theirs, and `mcp__tcl-lsp__analyze` and the server report the same codes for these fixtures (or the MCP tool documents that it omits the style pass).
- **Delete text comment scanners.** Route `inert_text::offset_in_comment` (`tcl-lsp-core/src/inert_text.rs:89`) and `first_command_head` (`tcl-compiler/src/analyser/commands/nested_invocations.rs:925`) through the token stream or `script_comment_facts`. Add a gate that fails on new `== '#'`/`== b'#'` comment tests outside `tcl-lexer` and `tcl-value-syntax`. Done when the census above lists only lexer owners.
- **`switch` body diagnostics.** Declare `switch`'s body as a pattern/body list value grammar in `tcl-registry`. Report an odd word count as an error (c03), and warn when a word starting with `#` is used as a pattern (c26, c04), with the tclsh hint text. Share the "has a comment" test between `tcl-cmd-core/src/switch.rs` and both engines' `cmd_switch.rs`. Done when c03 gets an error and c26 a warning in both the MCP tool and the server.
- **`#` in data lists.** Add a hint where a key/value or pattern/body list argument (`array set`, `dict create`, `foreach` lists and `switch`) has a word starting with `#` at the start of a line, using the same value-grammar declarations (c15, c24).
- **`#` in `expr` before 9.0.** Make the expression parser's error for `#` under `ExprCommentStyle::None` reach the analyser as a diagnostic (`tcl-lexer/src/expr_lexer.rs:849`, the analyser's expression checks). Done when c18 and t20 report under `tcl8.6` and not under `tcl9.0`.
- **Messages.** Report "missing close-brace" where tclsh does (c16, E200). Stop W123 suggesting `!` for ordinary words (c11, c20; fault 11). Make the VM's error location for a command spanning a backslash-space line match tclsh (c20: `line 1` and "invoked from within").
- **Docstrings.** Stop a doc comment at a swallowed line: show the comment text without the continued `proc hidden {} {}` line, or flag it (c25; `tcl-lexer/src/script.rs:230`, `tcl-lsp-core` hover).
- **Conformance rows.** Move the 27 fixtures into the conformance matrix with columns for tclsh 8.4–9.1, Jim, the VM, the runtime, the analyser codes (MCP and server), folding, and formatter and minifier equivalence (the `bcheck` method here: run the output and compare).
