# Interleaved registrations across two entities in one namespace.
set ::log {}
proc rec {label args} { lappend ::log $label }

namespace eval ::nsv {
    variable a 1
    variable b 2
}
trace add variable ::nsv::a unset {rec A1}
trace add variable ::nsv::b unset {rec B1}
trace add variable ::nsv::a unset {rec A2}
trace add variable ::nsv::b unset {rec B2}
namespace delete ::nsv
puts "var-interleaved: $::log"

set ::log {}
namespace eval ::nsc {
    proc x {} {}
    proc y {} {}
}
trace add command ::nsc::x delete {rec X1}
trace add command ::nsc::y delete {rec Y1}
trace add command ::nsc::x delete {rec X2}
trace add command ::nsc::y delete {rec Y2}
namespace delete ::nsc
puts "cmd-interleaved: $::log"

# Three entities, interleaved three deep.
set ::log {}
namespace eval ::ns3 {
    variable p 1
    variable q 2
    variable r 3
}
foreach n {1 2} {
    foreach v {p q r} {
        trace add variable ::ns3::$v unset [list rec $v$n]
    }
}
namespace delete ::ns3
puts "var-three: $::log"
