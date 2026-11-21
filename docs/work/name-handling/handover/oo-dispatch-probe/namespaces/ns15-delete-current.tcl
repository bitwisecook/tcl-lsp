namespace eval ::t { proc p {} { namespace delete [namespace current]; return [namespace exists ::t] } }
puts [::t::p]:[namespace exists ::t]
