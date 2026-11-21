namespace eval ::lib { namespace export get* set_*; proc get_a {} {return a}; proc getB {} {return B}; proc set_x {} {}; proc hidden {} {} }
namespace eval ::app { namespace import ::lib::get* }
puts [lsort [info commands ::app::*]]
puts [namespace eval ::lib {namespace export}]
namespace eval ::app { puts [namespace origin get_a] }
puts [catch {namespace eval ::app {namespace import ::lib::hidden}} m]:[info commands ::app::hidden]
namespace eval ::lib { proc get_a {} {return a2} }
puts [::app::get_a]
