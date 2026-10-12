proc hx {v} {binary scan $v H* result; return $result}
set name [binary format H* {@NAME@}]
proc probe {operand} {global $operand; set names [info -nons vars]; set i [lsearch -exact $names operand]; if {$i >= 0} {set names [lreplace $names $i $i]}; return $names}
set names [probe $name]
set outputs {}; foreach name $names {lappend outputs [hx $name]}; puts "[llength $names]|[join $outputs |]"
