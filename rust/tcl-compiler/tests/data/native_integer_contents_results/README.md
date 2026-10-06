# Original integer contents and normal results

`source.tcl` executes fixed-formal integer arithmetic with independent callers,
a trailing rest formal, an unrelated command rename, an original leading-zero
integer spelling, a constructed double argument, and native errors. Each output
contains the case name, actual completion code, and actual result.

The final two cases execute the same floating procedure before and after setting
`tcl_precision` to one. Tcl 8.4–8.6 use mutable thread-wide precision for double
string materialisation. Tcl 9 and Jim select their own immutable formats.
Known arithmetic values do not by themselves supply a floating result spelling.

The selected source integer-conversion receipt requires original accepted integer
contents and a closed stock-object lineage. It does not claim a current numeric
internal representation, and constructed `1.0` does not satisfy that receipt.
Unknown incoming values and observed or opaque object effects withhold it.
