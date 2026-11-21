namespace eval ::lib { proc h {} {return lib} }
namespace eval ::app { namespace path ::lib; proc r {} { h ;# CALL h
} }
puts [::app::r]
