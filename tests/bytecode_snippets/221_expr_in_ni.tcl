# ``expr {x in list}`` / ``x ni list`` must exact-match each list element
# against x, returning 0 or 1.  Regression for the compiler bug where
# the scan pass didn't pull in tcl_list_contains and _emit_binary fell
# through the "unsupported op" branch, so every ``in``/``ni`` returned 0.
if {![expr {5 in {1 2 3 4 5}}]} { error "5 in {...} lost" }
if {[expr {6 in {1 2 3 4 5}}]} { error "6 in reported true" }
if {![expr {6 ni {1 2 3 4 5}}]} { error "6 ni lost" }
if {[expr {"x" in {a b c}}]} { error "string-in false pos" }
if {[expr {"a" ni {a b c}}]} { error "string-ni false pos" }
return ok
