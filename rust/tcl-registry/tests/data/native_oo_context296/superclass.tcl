set available [expr {[llength [info commands oo::class]] > 0}]
if {!$available} {list NOT_APPLICABLE stock-TclOO-unavailable} else {
    oo::object create anobj
    set first [catch {oo::class create BadSuper {superclass anobj}} firstValue]
    set second [catch {oo::class create BadMixin {mixin anobj}} secondValue]
    list $first $firstValue $second $secondValue
}
