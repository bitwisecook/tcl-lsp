set code [catch {namespace eval ::a {variable value A; proc p {} {variable value; return $value}}; namespace eval ::b {variable value B}; rename ::a::p ::b::p; ::b::p} result]
binary scan $result H* hex
puts [list $code $hex]
