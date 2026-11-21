namespace eval ::lib { namespace export x; proc x {} {return lib} }
namespace eval ::app { proc x {} {return app}; namespace import -force ::lib::x; puts [x] ;# CALL x
}
