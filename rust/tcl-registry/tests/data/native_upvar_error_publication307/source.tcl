set rows {}
foreach {question body} {
    inverted {set x VALUE; upvar 0 x ::alias; set ::alias}
    local_element {set x VALUE; upvar 0 x alias(k); info exists alias}
    self {set x VALUE; upvar 0 x x; set x}
    existing {set x VALUE; set alias OLD; upvar 0 x alias; set alias}
    traced {set x VALUE; set alias OLD; trace variable alias r {list}; upvar 0 x alias; set alias}
} {
    proc p {} $body
    set ::errorCode SENTINEL
    set code [catch {p} result]
    lappend rows [list $question $code $result $::errorCode]
    rename p {}
}
set rows
