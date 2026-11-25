# chan postevent from a reflected channel's watch, delivered to chan event
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
proc rc {cmd id args} {
  switch -- $cmd {
    initialize { return {initialize finalize watch read} }
    watch { if {"read" in [lindex $args 0]} { after 0 [list chan postevent $id read] } ; return }
    read { return "x" }
    default { return }
  }
}
t create {set c [chan create read rc]; string match rc* $c}
t event {chan event $c readable {set ::got 1}; vwait ::got; set ::got}
t handler {chan event $c readable}
puts "bad-event: [catch {chan postevent $c sideways} m] $m"
t close {close $c}
