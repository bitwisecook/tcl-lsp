# In a script file, unknown does not exec external programs even with tcl_interactive set.
puts "tcl_interactive: [info exists tcl_interactive] [expr {[info exists tcl_interactive] ? $tcl_interactive : {-}}]"
set tcl_interactive 1
puts "echo: [catch {echo hi} m] $m"
puts "abbrev: [catch {stri length abc} m] $m"
