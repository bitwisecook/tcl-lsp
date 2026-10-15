# Parameter reference parts

`references.tcl` runs unchanged against original C Tcl 8.4.20, 8.5.19,
8.6.18, 9.0.4 and 9.1.0 with each release's original library, and pinned Jim.
`receipt.json` binds provider binaries, original init files, input and outputs.
No tcltest replacement or compiler implementation participates.

All six providers read `$1abc`, `${}` and `${odd name}` as scalar references.
The C providers read `$café` as `$caf` followed by literal `é`; Jim reads
`café`. C Tcl 9 uses the nested brace-name boundary for `${a{b}c}`; C Tcl 8
and Jim retain their first closer. `${arr($k)}` reads the literal array index
`$k`; it does not substitute `k`, whereas `$arr($k)` does.

Compiler parameter traits consume the original full word and configured
executable components. Literal braced words and escaped sigils supply no
parameter reference. A whole scalar and a component read remain distinct;
array names and substituted indices remain independently represented.

These observations establish reference grammar and value behaviour only.
They supply no binding, actual parameter value, entered caller frame, alias,
Normal completion or Native execution certificate. The two scalar-helper
controls for digit-leading names require a reference rather than absence;
their earlier identifier-only expectation does not match any observed provider.
