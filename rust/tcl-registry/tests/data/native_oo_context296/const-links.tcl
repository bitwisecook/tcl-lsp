if {![llength [info commands const]] || ![llength [info commands oo::class]]} {
    list NOT_APPLICABLE const-or-stock-TclOO-unavailable
} else {
    const G 9
    oo::class create C {variable X; constructor {} {const X 1}; method inspect {} {global G; upvar #0 ::G U; list [info consts] [info constant G] [info constant U]}; method retarget {} {upvar #0 ::G X; list [info consts] [info constant X]}; method shadow {X} {list $X [info consts]}}
    set o [C new]
    set a [catch {$o inspect} av]
    set b [catch {$o retarget} bv]
    set c [catch {$o shadow formal} cv]
    list $a $av $b $bv $c $cv
}
