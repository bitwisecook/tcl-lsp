proc p1 {} { try { error A } on error {m} { error B } }
proc p2 {} { try { error A } finally { error F } }
proc p3 {} { try { set x 1 } finally { error F } }
proc p4 {} { try { error A } on error {m} { error B } finally { error F } }
proc p5 {} { try { error A } trap {} {m o} { return -code error -errorinfo CUSTOM X } }
proc p6 {} {
    try {
        error A
    } on error {m} {
        set y 1
        error B
    }
}
set out ""
foreach p {p1 p2 p3 p4 p5 p6} {
    catch {$p} m o
    append out "== $p: $m\n[dict get $o -errorinfo]\n"
    if {[dict exists $o -during -errorinfo]} { append out "-- during: [dict get $o -during -errorinfo]\n" }
}
try { error A } on error {m} { catch {error B} m2 o2 }
append out "== top: [dict get $o2 -errorinfo]\n"
set out
