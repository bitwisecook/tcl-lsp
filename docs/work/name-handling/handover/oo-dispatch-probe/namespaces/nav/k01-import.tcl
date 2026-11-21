namespace eval ::lib { namespace export get*; proc get_a {} {return a} }
namespace eval ::app { namespace import ::lib::get*; puts [get_a] ;# CALL get_a
}
