# Original return words

`probe.tcl` runs directly under original C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4,
9.1.0 and JimTcl upstream commit `5bac7c99ad65864c87da513e22e2f01703fa4e03`.
The receipt records exact provider commands and executable hashes before/after,
matching original C library paths and init hashes, raw output hashes and exit
codes. The matching original C `init.tcl` files are retained unchanged. No
tcltest is used.

All six original providers agree: a braced `return {$x}` returns literal `$x`;
`return $a:b` reads `a` and appends literal `:b`, even when `a:b` has a distinct
value; `${arr(k)}` with scalar `arr` raises rather than reading that scalar.
These exact cases guard the optimiser's use of retained original Variable words
and the shared complete scalar-reference query. A successful scalar query still
supplies no independent read, storage, contents or Native dispatch authority.
Existing optimiser assertions, including positive ordinary scalar returns,
remain separate acceptance obligations. F5 remains unanswered, and the original
provider results do not claim Rust test acceptance.
