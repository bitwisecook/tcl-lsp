set code [catch {namespace eval ::n {variable stage BEFORE}; package ifneeded Demo 1.0 {namespace eval ::n {set stage LOADED; proc leaf {} {return INSTALLED}}; package provide Demo 1.0}; namespace eval ::n {list [package require Demo] $stage [leaf]}} result]
binary scan $result H* hex
puts [list $code $hex]
