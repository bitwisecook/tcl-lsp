if {[catch {info -commands} r2286_selectors] || [lsearch -exact $r2286_selectors alias] < 0} {
    set r2286_result NOT_APPLICABLE
} else {
    alias ::r2286_alias_query list FIRST SECOND
    set a [info alias r2286_alias_query]
    set b [info alias ::r2286_alias_query]
    set c [r2286_alias_query THIRD]
    list $a $b $c
}
