# Jim namespaces (if built in)
namespace eval ns {
    proc p {} {return [namespace current]}
    variable v 5
}
puts [ns::p]
puts [set ns::v]
puts [namespace exists ns]
