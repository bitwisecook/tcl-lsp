namespace eval ::lib { namespace export f; proc f {} {return libf} }
namespace eval ::app { namespace import ::lib::f }
rename ::lib::f ::lib::g
puts [list [::app::f] [namespace origin ::app::f]]
rename ::app::f ::app::h
puts [list [::app::h] [info commands ::app::*] [::lib::g]]
