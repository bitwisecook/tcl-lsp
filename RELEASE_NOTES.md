# v1.4.2

## New Features
- **W118 inconsistent line endings diagnostic**: new hint-level diagnostic
  that flags files with line endings differing from the configured style
  (LF vs CRLF), including detection of mixed endings within a single file.

## Bug Fixes
- Fixed W112 (trailing whitespace) false positive on CRLF line endings:
  the `\r` in `\r\n` was incorrectly counted as trailing whitespace
  (GH-95).
- Fixed W111 (line length) CRLF inflation: the `\r` in `\r\n` was
  counted as a character, causing lines at exactly the maximum length to
  be falsely flagged as too long.
- Fixed lexer handling of backslash-CR and backslash-CRLF sequences:
  `\<CR>` and `\<CR><LF>` are now correctly treated as line
  continuations in all contexts (unquoted words, double quotes, comments,
  and command substitutions), matching C Tcl's `TclParseBackslash`
  behaviour.
- Fixed `backslash_subst` to handle `\<CR>` and `\<CRLF>` continuation
  sequences, producing a single space like `\<LF>`.
- Fixed extract-proc code action CRLF handling: the refactoring no longer
  produces doubled line endings or fails to detect selection boundaries
  in files with Windows-style line endings.
