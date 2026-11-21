namespace eval ::a { proc p {} {return p} }
namespace eval ::b {}
rename ::a::p ::b::q
puts [list [info commands ::a::*] [info commands ::b::*] [catch {rename ::b::q ::nons::q} m] $m [namespace exists ::nons] [info commands ::nons::*]]
namespace eval ::nons { rename q r }
puts [info commands ::nons::*]
