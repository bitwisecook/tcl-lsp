unset -nocomplain {}
proc mutate_empty {} {global {}; append {} VALUE}
puts [list before [info exists {}]]
puts [list code [catch {mutate_empty} result] result $result]
puts [list after [info exists {}]]
if {[info exists {}]} {puts [list value [set {}]]}
set {$literal} ORIGINAL
proc mutate_sigil {} {global {$literal}; append {$literal} +VALUE}
puts [list sigil_code [catch {mutate_sigil} result] result $result]
puts [list sigil_value [set {$literal}]]
puts [list bare_exists [info exists literal]]
