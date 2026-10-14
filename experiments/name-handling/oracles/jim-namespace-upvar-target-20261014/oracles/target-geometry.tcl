rename upvar original_upvar
proc upvar {args} {return $args}
foreach {ns other} {{} x ::N x ::N ::x ::N {} :: {} } {
    puts [list $ns $other [namespace upvar $ns $other local]]
}
namespace eval outer {puts [namespace upvar child x local]}
set ns "N\u0000tail"
set other "x\u0000tail"
puts [namespace upvar $ns $other local]
