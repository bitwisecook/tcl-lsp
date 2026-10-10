proc row {name script} {
    set code [catch {uplevel 1 $script} result]
    set length [string length $result]
    set units {}
    for {set index 0} {$index < $length} {incr index} {
        scan [string index $result $index] %c unit
        lappend units $unit
    }
    puts [list $name $code $length $units]
}
row exported-distinct-source-units {
    namespace eval ::Scope88A {
        proc p\uD800 {} {return FIRST}
        proc p\uD801 {} {return SECOND}
        namespace export p\uD800 p\uD801
    }
    namespace eval ::Scope88B {
        namespace import ::Scope88A::p\uD800 ::Scope88A::p\uD801
        list [p\uD800] [p\uD801]
    }
}
row export-filter-keeps-distinct-units {
    namespace eval ::Scope88FilterA {
        proc p\uD800 {} {return FIRST}
        proc p\uD801 {} {return SECOND}
        namespace export p\uD800
    }
    namespace eval ::Scope88FilterB {
        namespace import ::Scope88FilterA::*
        list [llength [info commands p\uD800]] [llength [info commands p\uD801]]
    }
}
row import-source-move-identity {
    namespace eval ::Scope88MoveA {
        proc p {} {return HELD}
        namespace export p
    }
    namespace eval ::Scope88MoveB {namespace import ::Scope88MoveA::p}
    rename ::Scope88MoveA::p ::Scope88MoveA::q
    ::Scope88MoveB::p
}
row forget-local-distinct-units {
    namespace eval ::Scope88B {
        namespace forget p\uD800
        list [llength [info commands p\uD800]] [p\uD801]
    }
}
row forget-renamed-import-by-origin {
    namespace eval ::Scope88OriginA {
        proc p\uD800 {} {return HELD}
        namespace export p\uD800
    }
    namespace eval ::Scope88OriginB {namespace import ::Scope88OriginA::p\uD800}
    rename ::Scope88OriginB::p\uD800 ::Scope88OriginB::moved
    namespace eval ::Scope88OriginB {
        namespace forget ::Scope88OriginA::p\uD800
        llength [info commands moved]
    }
}
row import-invalid-no-source {
    namespace eval ::Scope88Invalid {namespace import local}
}
row import-missing-source {
    namespace eval ::Scope88Invalid {namespace import ::NoScope88::p}
}
row import-same-namespace {
    namespace eval ::Scope88A {namespace import ::Scope88A::p\uD801}
}
row export-qualified-invalid {
    namespace eval ::Scope88A {namespace export ::Scope88A::p\uD801}
}
row force-replacement-order {
    namespace eval ::Scope88ForceA {
        proc p {} {return SOURCE}
        namespace export p
    }
    namespace eval ::Scope88ForceB {
        proc p {} {return DESTINATION}
        set before [catch {namespace import ::Scope88ForceA::p} result]
        set retained [p]
        namespace import -force ::Scope88ForceA::p
        list $before $retained [p]
    }
}
row export-leading-clear {
    namespace eval ::Scope88ClearA {
        proc before {} {return BEFORE}
        proc after {} {return AFTER}
        namespace export before
        namespace export -clear after
    }
    namespace eval ::Scope88ClearB {
        namespace import ::Scope88ClearA::*
        list [llength [info commands before]] [after]
    }
}
