# chan create with every optional method; seek/tell, configure/cget/cgetall, blocking, truncate (8.6: truncate from 8.5?)
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
set ::log {}
proc rc {cmd id args} {
  lappend ::log [list $cmd {*}$args]
  switch -- $cmd {
    initialize { return {initialize finalize watch read write seek configure cget cgetall blocking truncate} }
    read { return "" }
    write { return [string length [lindex $args 0]] }
    seek { return 42 }
    cget { return val }
    cgetall { return {-foo val} }
    default { return }
  }
}
t create {set c [chan create {read write} rc]; string match rc* $c}
t tell {tell $c}
t seek {seek $c 10 start; tell $c}
t puts {puts -nonewline $c abc; flush $c}
t cget {chan configure $c -foo}
t set {chan configure $c -foo bar}
t blocking {chan configure $c -blocking 0}
t truncate {chan truncate $c 5}
t cgetall {lsort [dict keys [chan configure $c]]}
t close {close $c}
t log {join $::log |}
