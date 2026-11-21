namespace eval ::d { proc p {} {return old} }
namespace delete ::d
puts [catch {::d::p} m]:$m ;# CALL p
