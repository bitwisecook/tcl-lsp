# Original braced script correspondence

How do the real supported interpreters retain ordinary occurrences around
braced line continuations, escaped-backslash pairs, preceding literal spaces,
and a CRLF-containing script supplied as an evaluated value?

The script uses each original interpreter startup and no Tcltest harness.
C source files use the appropriate distribution's `TCL_LIBRARY`; the native
CRLF case supplies an evaluated script value independently of file-channel
newline handling. Static value observations do not establish Rust mapping,
procedure entry, variable-cell contents or edit authority.
