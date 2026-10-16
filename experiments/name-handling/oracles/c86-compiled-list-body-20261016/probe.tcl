namespace eval test_ns_var {variable foo 2}
proc p {} {
    variable ::test_ns_var::foo
    namespace delete ::test_ns_var
    set command [list set foo(3) 3]
    puts [list BEFORE [::tcl::unsupported::representation $command]]
    set code [catch $command result]
    puts [list AFTER [::tcl::unsupported::representation $command]]
    puts [list ROW $code $result $::errorCode]
}
p
puts [::tcl::unsupported::disassemble proc p]
