# Original braced value observations

The five original C Tcl distributions 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0
agree for the LF and CRLF input files: one braced backslash-newline folds to
one space, an escaped-backslash pair remains literal, and preceding spaces
remain. An evaluated script containing backslash-CRLF retains those bytes;
file-channel newline handling is a separate input path.

JimTcl at commit `5bac7c99ad65864c87da513e22e2f01703fa4e03` retains braced
continuations and their following whitespace, including original CRLF file
bytes. Its escaped-backslash and evaluated native-CRLF controls remain
literal. These results select the corresponding source/brace recipes; they
supply no native Rust entry, variable-cell or edit authority.

Both source files, original raw streams, interpreter executable hashes, exact
commands, startup library selectors, elapsed times and exit codes are in
`results.json`. No Tcltest harness is used. The compiler mapping tests use
these byte observations and independent exact source-boundary controls.
