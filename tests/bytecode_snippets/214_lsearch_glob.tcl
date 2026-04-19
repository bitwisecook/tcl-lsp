# Tcl ``lsearch`` default match mode is glob, not exact.  Regression
# for the runtime bug where ``tcl_cmd_list_search`` did a byte-for-byte
# compare and returned -1 for any list/pattern pair unless the pattern
# exactly equalled an element.
set x {abc bdef abcdef bcdefg}
if {[lsearch $x bcd*] != 3} { error "glob prefix match failed" }
if {[lsearch $x *cd*] != 0} { error "wildcard middle match failed" }
if {[lsearch $x nosuch] != -1} { error "no-match should return -1" }
# Exact text still matches as a trivial glob.
if {[lsearch {a b c d} c] != 2} { error "exact-as-glob failed" }
set x
