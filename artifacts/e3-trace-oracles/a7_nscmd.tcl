set ::log {}
proc rec {label args} { lappend ::log $label }
namespace eval ::doomed {
    proc victim {} {}
}
trace add command ::doomed::victim delete {rec first}
trace add command ::doomed::victim delete {rec second}
namespace delete ::doomed
puts "ns-cmd-delete: $::log"
