proc hx v {binary scan $v H* out; return $out}
proc f args {tailcall target [set seen VALUE]}
set code [catch {::tcl::unsupported::disassemble proc f} text]
puts "tailcall\t7461696c63616c6c20746172676574205b736574207365656e2056414c55455d\t$code\t[hx $text]"
proc f args {tailcall}
set code [catch {::tcl::unsupported::disassemble proc f} text]
puts "tailcall-empty\t7461696c63616c6c\t$code\t[hx $text]"
proc f args {tailcall {*}$args}
set code [catch {::tcl::unsupported::disassemble proc f} text]
puts "tailcall-expand\t7461696c63616c6c207b2a7d2461726773\t$code\t[hx $text]"
proc f args {yield}
set code [catch {::tcl::unsupported::disassemble proc f} text]
puts "yield-empty\t7969656c64\t$code\t[hx $text]"
proc f args {yield [set seen VALUE]}
set code [catch {::tcl::unsupported::disassemble proc f} text]
puts "yield\t7969656c64205b736574207365656e2056414c55455d\t$code\t[hx $text]"
proc f args {yield a b}
set code [catch {::tcl::unsupported::disassemble proc f} text]
puts "yield-too-many\t7969656c6420612062\t$code\t[hx $text]"
proc f args {yieldto target [set seen VALUE]}
set code [catch {::tcl::unsupported::disassemble proc f} text]
puts "yieldto\t7969656c64746f20746172676574205b736574207365656e2056414c55455d\t$code\t[hx $text]"
proc f args {yieldto}
set code [catch {::tcl::unsupported::disassemble proc f} text]
puts "yieldto-empty\t7969656c64746f\t$code\t[hx $text]"
proc f args {yieldto {*}$args}
set code [catch {::tcl::unsupported::disassemble proc f} text]
puts "yieldto-expand\t7969656c64746f207b2a7d2461726773\t$code\t[hx $text]"
proc target args {return [list [namespace current] $args]}
proc issue {} {tailcall target [set seen VALUE]}
set code [catch {issue} result]
puts "runtime\t$code\t[hx $result]"
