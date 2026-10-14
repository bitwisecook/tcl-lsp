puts [catch {proc ::b::run {} {global bbb; set bbb 1}}]
namespace eval ::b {}; namespace eval ::a {}
proc ::b::run {} {global bbb; set bbb 1}
proc ::a::run {} {global aaa; set aaa 1}
::b::run; ::a::run
puts [list $bbb $aaa]
