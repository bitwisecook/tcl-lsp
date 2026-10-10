if {[catch {info -commands} r2286_selectors] || [lsearch -exact $r2286_selectors alias] < 0} {
    set r2286_result NOT_APPLICABLE
} else {
    alias ::r2286_alias_a list A
    alias r2286_holder::r2286_alias_b list B
    namespace eval r2286_holder {
        set a [catch {info aliases -all r2286*} ar]
        set b [catch {info -nons aliases -all r2286*} br]
        list $a $ar $b $br
    }
}
