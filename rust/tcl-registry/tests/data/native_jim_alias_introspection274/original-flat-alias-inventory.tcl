if {[catch {info -commands} r2286_selectors] || [lsearch -exact $r2286_selectors alias] < 0} {
    set r2286_result NOT_APPLICABLE
} else {
    proc r2286_proc {} {return PROC}
    alias ::r2286_alias_a list A
    alias {::r2286 spaced alias} list SPACE
    alias r2286_holder::r2286_alias_b list B
    list [lsort [info aliases r2286*]] [lsort [info aliases -all r2286*]] [lsort [info aliases -all r2286_holder::*]]
}
