# Tcl VM array-index grammar lane

## Goal

Fix #1732 against Tcl 9.0.4: Tcl 9.0 and 9.1 reject an unescaped grammar
character in the source of a `$name(index)` read, while Tcl 8.4–8.6 accept
those bytes as index text. Escaped characters and values introduced by
variable or command substitution remain valid. The store-side spelling
`set a({k}) value` is an ordinary command word and remains legal in every
release.

## Design

- Add one named release axis to `tcl_dialect::LexerGrammar`. The axis owns the
  C `ParseTokens` mask used for array-index source and exposes the byte
  classifier consumed everywhere.
- Thread that axis through `tcl_lexer::LexerConfig`. The main script lexer,
  expression lexer, and Rust runtime parser must consult the same fact after
  skipping escaped bytes and complete variable/command substitutions.
- Surface the exact `invalid character in array index` text from the shared
  lexer owner. The compiler's existing fatal-parse barrier must classify it as
  fatal so bytecode hosts defer it as a catchable runtime error just like C.
- Preserve lenient token recovery for editor consumers: record a warning at
  the first invalid source byte, but continue the variable token through its
  closing parenthesis.

## Site inventory

- [ ] `rust/tcl-dialect`: release axis, catalogue/dynamic grammar values, and
  invariant tests.
- [ ] `rust/tcl-lexer`: `LexerConfig` threading, main array scanner, expression
  scanner, shared error spelling, and release/control tests.
- [ ] `rust/tcl-compiler`: fatal parse barrier classification and tests.
- [ ] `runtime/rust`: `scan_parts_at_depth` / script-word runtime parity and
  focused execution tests.
- [ ] `rust/tcl-vm`: cross-version bytecode VM and Tcl oracle coverage.
- [ ] ownership/design documentation and mutation verification.

## Oracle evidence

The exact Tcl 9 oracle is
`/home/jimd/src/tcl9.0.4/unix/tclsh` with
`LD_LIBRARY_PATH=/home/jimd/src/tcl9.0.4/unix`. Tcl 9.0.4 rejects
`$a({k})` with `invalid character in array index`, but accepts `$a(\{k\})`,
`$a($index)`, and `$a([set index])` when those substitutions produce `{k}`.
System Tcl 8.6.17 accepts all four reads.

Upstream source evidence is `generic/tclParse.h`'s
`TYPE_BAD_ARRAY_INDEX` and `generic/tclParse.c`'s `Tcl_ParseVarName` call to
`ParseTokens`. Tcl 8.6 passes only `TYPE_CLOSE_PAREN`; Tcl 9.0.4 also stops on
raw open parenthesis, quote, and either brace, then raises the exact message.

## Open uncertainties

- The runtime parser currently represents other malformed substitutions as
  `WordPart::ParseError` so evaluation can raise them. Verify whether the new
  script-word error needs the same representation or can be rejected earlier
  from the lexer warning without changing public command structures.
- The requested regression is raw braces, but the named Tcl 9 axis should
  model C's complete source mask rather than baking two brace checks into
  three scanners. Add controls for the complete mask without broadening the
  issue's behavioural claims beyond verified C source.
