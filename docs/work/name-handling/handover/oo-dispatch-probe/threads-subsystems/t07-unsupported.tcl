# tcl::unsupported:: commands that exist per release.
puts "cmds=[lsort [lmap c [info commands ::tcl::unsupported::*] {namespace tail $c}]]"
set x [expr {1 + 2}]
puts "repr=[string match {value is a int*} [tcl::unsupported::representation $x]]"
proc p {} { return 1 }
puts "disasm=[string match *ByteCode* [tcl::unsupported::disassemble proc p]]"
puts "assemble=[tcl::unsupported::assemble {push 5; push 6; add}]"
coroutine co apply {{} { yield; return }}
puts "corotype=[tcl::unsupported::corotype co]"
