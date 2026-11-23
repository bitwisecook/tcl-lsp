# tcl_wordchars / tcl_nonwordchars appear after word.tcl is auto-loaded.
puts "before=[info exists tcl_wordchars]/[info exists tcl_nonwordchars]"
puts "break=[tcl_wordBreakAfter {ab cd} 0]"
puts "after=[info exists tcl_wordchars]/[info exists tcl_nonwordchars]"
