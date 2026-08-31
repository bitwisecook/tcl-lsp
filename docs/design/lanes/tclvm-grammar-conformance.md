# Tcl VM grammar conformance lane

## Goal and issue boundary

This lane makes Tcl 9.0.4 the grammar oracle for two parser-foundation bugs:

- **#1579 — raw backslash/CR handling.** Complete in this lane. C's raw
  `TclParseBackslash` continuation is `\\<LF>` only. A channel may translate
  CRLF before parsing, but raw `\\<CR>` and `\\<CRLF>` passed to the parser are
  ordinary escaped data.
- **#1580 — unique boolean prefixes in expressions.** Complete in this lane.
  Identifier-shaped expression tokens use the existing
  `tcl_syntax::boolean::parse_boolean_word` owner after function-call
  precedence has been resolved. The accepted prefix set is release-invariant
  across every supported Tcl release.
- **#1732 — literal braces in array-index reads.** Deliberately not implemented
  here. It needs a distinct release-aware grammar axis and fatal-parse error
  path; folding it into the two release-invariant fixes would create another
  implicit owner and entangle #1586/#1603 recovery and diagnostics.

## Owners and consumers

The one raw continuation classifier is
`tcl_lexer::backslash_continuation_end`. `backslash_subst`, escape-span
scanning, the main lexer, expression whitespace lexing, brace collapse,
structural recovery, script completeness, and command-boundary indexing all
consume it or the canonical escape-width function beside it. Runtime parsing
therefore inherits the same rule through `tcl-lexer` rather than carrying a
second CR/CRLF table.

The one boolean-word classifier remains
`tcl_syntax::boolean::parse_boolean_word`. Both expression parser entry points
consult it, after the C-compatible `identifier + (` function-call precedence.
The VM already coerces through that owner; the Rust WASM runtime now does too.
Literal evaluation preserves the source spelling (`expr {yes}` returns
`yes`), and coercion to `1`/`0` happens only in a boolean context.

## Tcl 9.0.4 oracle and fixed regressions

Gold interpreter:

```text
LD_LIBRARY_PATH=/home/jimd/src/tcl9.0.4/unix \
  /home/jimd/src/tcl9.0.4/unix/tclsh
```

Focused upstream suite:

```text
cd /home/jimd/src/tcl9.0.4/unix
LD_LIBRARY_PATH=$PWD ./tclsh ../tests/all.tcl -singleproc 1 \
  -file expr.test -match 'expr-21.* expr-31.*'
```

This selects the non-numeric boolean literal and boolean-conversion families:
80 passed, 0 failed. The analogous `parse.test` selection
(`parse-1.5`, `parse-1.6`, `parse-1.7`, `parse-1.9`, `parse-1.10`,
`parse-2.3`, `parse-2.4`, `parse-5.1`, `parse-5.2`, `parse-5.7`,
`parse-5.9`, `parse-6.13`–`parse-6.15`, and `parse-14.7`–`parse-14.10`)
requires the optional `testparser` extension and was skipped by this build, so
the lane pins raw byte-vector oracle calls as ordinary deterministic Rust tests
instead of pretending the skipped rows ran.

Direct 9.0.4 and secondary `/usr/bin/tclsh9.0` 9.0.3 checks agree:

```text
expr {tru}, expr {y}, expr {of} -> original spelling
expr {o}                       -> invalid bareword
"a\\<CR>Z"                    -> hex 610d5a
"a\\<CR><LF>Z"                -> hex 610d0a5a
"a\\<LF>Z"                    -> hex 61205a
```

Deepest-owner tests live in `tcl-lexer` (continuation extent) and
`tcl-syntax` (boolean recognition/parser). Integration tests cover the main
lexer and structural index, the bytecode VM over every `TclVersion`, and the
Rust runtime with its Tcl 9.0.4 libtommath source.

## #1732 follow-up contract

The follow-up must add a named release-axis fact to
`tcl_dialect::LexerGrammar` and thread it into `tcl_lexer::LexerConfig`. Tcl
8.4–8.6 accept literal `{` and `}` source bytes in a `$name(index)` read; Tcl
9.0–9.1 reject them with `invalid character in array index`. Escaped braces
and braces introduced by substitution remain valid. The rule applies to
read-side `$name(index)` grammar, not an arbitrary parenthesised command word.

One owner must drive all three scanners:

- `rust/tcl-lexer/src/lexer.rs::scan_array_index_body`;
- `rust/tcl-lexer/src/expr_lexer.rs::scan_array_index`;
- `runtime/rust/src/parse.rs::scan_parts_at_depth`.

The compiler must then surface the owner result through its existing fatal
parse diagnostic/lowering barrier, with exact 8.x versus 9.x tests. Do not add
three local checks for brace bytes. The regression matrix must include raw
`{`/`}` rejection in 9.x, acceptance in 8.x, escaped braces in both, a brace
arriving through `$key` in both, write-side controls, and expression/main
script/runtime consumers. That error-timing work should be coordinated with
#1586/#1603 rather than hidden in a lenient lexer recovery branch.

## Remaining work in this lane

- Run the focused crate checks and clippy under the worktree-isolated build
  environment.
- Mutation-verify the LF-only continuation branch and the boolean-prefix
  acceptance branch.
- Keep commits local; the orchestrator owns push, PR, and merge.
