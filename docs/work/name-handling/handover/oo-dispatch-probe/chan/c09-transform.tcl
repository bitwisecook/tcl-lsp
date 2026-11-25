# chan push / chan pop transform: call order and arguments
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
set ::log {}
proc xf {cmd id args} {
  lappend ::log $cmd
  ;# COMPLETE initialize
  switch -- $cmd {
    initialize { return {initialize finalize read write flush drain clear} }
    read - write { return [string toupper [lindex $args 0]] }
    default { return }
  }
}
t write {set f [open c09.txt w]; chan push $f xf; puts $f hello; chan pop $f; close $f}
t raw {set f [open c09.txt r]; set r [read $f]; close $f; set r}
t read {set f [open c09.txt r]; chan push $f xf; set r [gets $f]; close $f; set r}
t log {set ::log}
