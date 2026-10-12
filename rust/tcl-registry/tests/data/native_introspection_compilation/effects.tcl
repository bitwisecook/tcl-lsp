namespace eval N {}
proc arrayObserver {name key op} {lappend ::events ARRAY}
proc values {odd} {lappend ::events RHS; if {$odd} {return k}; return {k V}}
foreach mode {even odd literalodd empty} {
    set ::events {}
    proc p {} [format {trace add variable a array arrayObserver; %s} [lindex { {array set a [values 0]} {array set a [values 1]} {array set a {k}} {array set a {}} } [lsearch -exact {even odd literalodd empty} $mode]]]
    set code [catch {p} result]
    puts [list arrayeffects $mode $code $::events $result]
}
foreach value {2147483648 -2147483649 invalid} {
    proc p {value} {info level $value}
    set code [catch {p $value} result]
    puts [list levelerror $value $code $result $::errorCode]
}
