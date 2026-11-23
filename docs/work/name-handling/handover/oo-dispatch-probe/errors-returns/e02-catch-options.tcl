# catch with an options variable: keys and values for each code.
proc keys {d} { set out {}; foreach k [lsort [dict keys $d]] { if {$k ne "-errorinfo" && $k ne "-errorstack"} { lappend out $k [dict get $d $k] } }; return $out }
catch {set x 1} r o; puts "ok: [keys $o]"
catch {error boom INFO {MY CODE}} r o; puts "error: [keys $o]"
catch {return -level 2 -code break x} r o; puts "return: [keys $o]"
catch {break} r o; puts "break: [keys $o]"
catch {return -code 7 -foo bar seven} r o; puts "custom: [keys $o]"
puts "info: [string map [list \n |] [dict get [list {*}[catch {error boom INFO} r o; set o]] -errorinfo]]"
