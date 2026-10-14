set g BEFORE
proc root_link {} {upvar #0 g local; set local GLOBAL}
root_link
puts [list GLOBAL [set g]]
proc literal_link {} {upvar 1 {$literal} {}; set {} LITERAL}
proc literal_caller {} {set {$literal} BEFORE; literal_link; puts [list LITERAL [set {$literal}]]}
literal_caller
proc implicit_link {} {upvar destination local; set local DEFAULT}
proc implicit_caller {} {set destination BEFORE; implicit_link; puts [list DEFAULT $destination]}
implicit_caller
