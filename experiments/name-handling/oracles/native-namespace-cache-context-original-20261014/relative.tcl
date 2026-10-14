puts [list VERSION [info patchlevel]]
namespace eval ::Target {}
namespace eval ::Left {namespace eval Target {}; namespace eval :Target {}}
namespace eval ::Right {namespace eval Target {}; namespace eval :Target {}}
set ::operand Target
set left [namespace eval ::Left {namespace eval $::operand {namespace current}}]
set right [namespace eval ::Right {namespace eval $::operand {namespace current}}]
set again [namespace eval ::Left {namespace eval $::operand {namespace current}}]
puts [list CONTEXTS $left $right $again]
namespace delete ::Target
namespace eval ::Target {}
set renewed [namespace eval ::Right {namespace eval $::operand {namespace current}}]
puts [list RECREATED $renewed]
