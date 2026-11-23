# auto_execok finds executables on PATH; empty when absent.
puts "sh: [expr {[auto_execok sh] ne {} ? {found} : {missing}}]"
puts "none: [list [auto_execok no_such_program_xyz]]"
puts "builtin: [list [auto_execok cd]]"
