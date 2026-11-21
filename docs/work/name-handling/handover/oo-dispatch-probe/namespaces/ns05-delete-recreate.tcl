namespace eval ::d { variable v 1; proc p {} {return old} }
namespace eval ::u { namespace import ::d::* }
namespace eval ::d { namespace export p }
namespace eval ::u { namespace import ::d::p }
namespace delete ::d
puts [list [namespace exists ::d] [info commands ::u::p] [catch {set ::d::v} m] [info procs ::d::*]]
namespace eval ::d { proc p {} {return new} }
puts [list [info commands ::u::p] [::d::p]]
