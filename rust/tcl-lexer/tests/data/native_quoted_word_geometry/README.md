# Original quoted word geometry

`probe.c` calls each C Tcl release's public `Tcl_ParseCommand` and reports
case index, word count, the final complete word end and source length. All
five cases retain the closing delimiter, including quoted words whose final
fragment is a backslash escape and bare words ending in a bracketed fragment.
`probe.tcl` evaluates the original TMM operator expression on each C release
and current Jim Tcl. The manifest records engine, source, header, library and
output hashes with the per-process timeout.

The shared lexer captures this geometry in `NativeWord::from_group` and
`native_script_words_in`. Native compiler source adapters authenticate complete
original vectors against these owners; missing or mismatched original source
provides no compiler preparation receipt.
