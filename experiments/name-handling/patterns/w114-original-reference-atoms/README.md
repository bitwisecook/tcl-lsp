The fixed `references.tcl` input compares nested and direct expression values
under original C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 and the pinned Jim
interpreter. `receipt.json` retains actual executable/input/output digests and
capture times. Each process uses its own original Tcl library; no `tcltest`
package or substitute initialization script is involved.

Whole scalar references preserve spaces, literal sigils and the empty name.
C 9 accepts nested braces where earlier C and Jim close at the first brace;
Jim accepts an unbraced Unicode name where C rejects it. Array references are
observed separately and remain outside the scalar-only rewrite shortcut.
The diagnostic still marks expression unwrapping as requiring review: these
name/parser observations establish no general numeric-formatting equivalence,
entered lookup, variable cell or Native execution authority.
