namespace eval ::lib { namespace export f; proc f {} {return libf} }
namespace eval ::app { namespace import ::lib::f }
rename ::lib::f ::lib::g
puts [::app::f] ;# CALL f
