# compile errors and their text
foreach re {( a** {a{3,2}} {[z-a]} {\1} {a{1000}} (?z) {x\k} {[[:foo:]]} *a {a|*}} {
    if {[catch {regexp $re abc} m]} { puts "$re: $m | [lrange $::errorCode 0 2]" } else { puts "$re: ok $m" }
}
