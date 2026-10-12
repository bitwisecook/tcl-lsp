puts "PATCHLEVEL [info patchlevel]"
set code [catch {
proc p {} {return ROOT}; namespace eval n {proc p {} {return INNER}; namespace eval child {proc q {} {return CHILD}}}; namespace eval D {namespace import ::n::p}; list [namespace canonical] [namespace eval n {namespace canonical}] [lsort [namespace eval n {info procs *}]] [namespace eval n {info commands p}] [lsort [namespace eval n {info procs ::n::*}]] [namespace eval n {namespace which -variable absent}] [namespace eval n {set value LOCAL; namespace eval child {set value CHILD}; set value}] [D::p] [namespace origin D::p]
} value]
binary scan $value H* hex
puts "OBSERVATION $code $hex"
