namespace eval a: {proc p {} {return LEFT}}
namespace eval a {proc :p {} {return RIGHT}}
puts [list contexts [namespace eval a: {namespace current}] [namespace eval a {namespace current}]]
puts [list first [namespace eval a: {p}] second [namespace eval a {:p}]]
puts [list display-left [namespace eval a: {namespace which -command p}] display-right [namespace eval a {namespace which -command :p}]]
namespace eval a: {proc p {} {return REPLACED}}
puts [list after-left [namespace eval a: {p}] untouched-right [namespace eval a {:p}]]
