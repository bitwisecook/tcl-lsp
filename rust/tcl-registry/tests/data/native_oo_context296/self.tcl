set available [expr {[llength [info commands oo::class]] > 0}]
if {!$available} {list NOT_APPLICABLE stock-TclOO-unavailable} else {
    oo::class create Cls
    Cls create obj
    set c [catch {oo::define Cls {self}} value]
    set o [catch {oo::objdefine obj {self}} object]
    list $c $value $o $object
}
