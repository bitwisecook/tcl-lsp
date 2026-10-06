proc p {} {catch {error BODY} m o; list [dict get $o -errorstack] [dict keys $o]}
puts "caught|[p]"
proc q {} {error BODY}
catch {q} m o
puts "unwound|[list [dict get $o -errorstack] [dict keys $o]]"
proc r {} {return -code error BODY}
catch {r} m o
puts "return|[list [dict get $o -errorstack] [dict keys $o]]"
proc u {} {catch {uplevel 1 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]}
puts "shifted|[u]"
proc e {} {catch {return -level 0 -code error -options {-custom kept -errorcode CUSTOM} BODY} m o; list [dict get $o -errorstack] [dict keys $o]}
puts "explicit|[e]"
