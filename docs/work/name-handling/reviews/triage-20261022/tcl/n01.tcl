namespace eval a: {proc p {} {return LEFT}}
namespace eval a  {proc :p {} {return RIGHT}}
puts [namespace eval a: {p}]
puts [namespace eval a  {:p}]
