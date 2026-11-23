# chan create: order and arguments of handler calls for a read-only reflected channel
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
set ::log {}
set ::data "line1\nline2\n"
proc rc {cmd args} {
  lappend ::log [list $cmd {*}[lrange $args 1 end]]
  switch -- $cmd {
    initialize { return {initialize finalize watch read} }
    finalize { return }
    watch { return }
    read { lassign $args id n; set r [string range $::data 0 [expr {$n-1}]]; set ::data [string range $::data $n end]; return $r }
  }
}
t create {set c [chan create read rc]; string match rc* $c}
t gets {gets $c}
t cfg {lsort [dict keys [chan configure $c]]}
t close {close $c}
t log {set ::log}
