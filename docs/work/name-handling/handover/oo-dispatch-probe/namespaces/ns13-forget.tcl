namespace eval ::lib { namespace export f g; proc f {} {}; proc g {} {} }
namespace eval ::app { namespace import ::lib::* ; namespace forget ::lib::f }
puts [lsort [info commands ::app::*]]
namespace eval ::app { rename g gg }
puts [list [lsort [info commands ::app::*]] [namespace origin ::app::gg]]
