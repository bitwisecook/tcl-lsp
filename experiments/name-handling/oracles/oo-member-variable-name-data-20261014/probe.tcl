if {[llength [info commands ::oo::class]] == 0} {
    puts TCL_OO_UNAVAILABLE
} else {
    oo::class create NameData {
        variable {$literal} {${braced}}
        constructor {} {set {$literal} ONE; set {${braced}} TWO}
        method values {} {list [set {$literal}] [set {${braced}}] [info exists literal] [info exists braced]}
    }
    set object [NameData new]
    puts [list values [$object values]]
    puts [list names [info class variables NameData]]
    $object destroy
    NameData destroy
}
