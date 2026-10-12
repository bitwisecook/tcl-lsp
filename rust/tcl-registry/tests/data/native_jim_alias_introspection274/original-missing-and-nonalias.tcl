if {[catch {info -commands} r2286_selectors] || [lsearch -exact $r2286_selectors alias] < 0} {
    set r2286_result NOT_APPLICABLE
} else {
    proc r2286_plain {} {return PROC}
    set a [catch {info alias r2286_absent} ar]
    set b [catch {info alias r2286_plain} br]
    set c [catch {info alias {::r2286 spaced absent}} cr]
    list $a $ar $b $br $c $cr
}
