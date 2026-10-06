proc hx {v} {binary scan $v H* result; return $result}
proc probe {operand} {set $operand VALUE; set cells [info -nons vars]; set p [lsearch -exact $cells operand]; set cells [lreplace $cells $p $p]; if {[llength $cells] != 1} {error "unexpected variables: $cells"}; set root [lindex $cells 0]; set value [set $root]; set status [catch {dict keys $value} keys]; if {$status} {return "[hx $root]|SCALAR"}; if {[llength $keys] != 1} {error "unexpected dictionary keys: $keys"}; return "[hx $root]|[hx [lindex $keys 0]]"}
puts [probe [binary format H* {@NAME@}]]
