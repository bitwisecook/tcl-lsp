proc ::helper {} { return ::helper }
namespace eval ::ns {
    proc helper {} { return ::ns::helper }
    oo::class create C { method m {} { helper ;# CALL
    } }
}
puts [[::ns::C new] m]
