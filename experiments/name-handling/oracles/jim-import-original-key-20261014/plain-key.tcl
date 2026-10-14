namespace eval src {proc x {} {return OLD}}
namespace eval dest {namespace import ::src::x}
puts [list imported [info alias dest::x] qualifiers [namespace qualifiers [lindex [info commands src::x] 0]]]
set code [catch {namespace eval src {namespace import ::src::x}} result]
puts [list self $code $result]
