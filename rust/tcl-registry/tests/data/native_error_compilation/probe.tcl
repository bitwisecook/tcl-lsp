set bodies [list {error BODY} {error [set message BODY] [set info STACK]} {error [set message BODY] [set info STACK] [set code {FOO BAR}]} {error {*}{BODY STACK {FOO BAR}}} {error}]
set n 0
foreach body $bodies {
 proc p {} $body
 puts "CASE $n SOURCE $body"
 if {[llength [info commands ::tcl::unsupported::disassemble]]} {
   set dc [catch {::tcl::unsupported::disassemble proc p} dis]
   puts "DISASSEMBLY $dc $dis"
 }
 set c [catch {p} value]
 puts "RESULT $c $value"
 incr n
}
