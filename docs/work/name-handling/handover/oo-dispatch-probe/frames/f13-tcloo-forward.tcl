proc ::helper {} { return ::helper }
namespace eval ::ns {
    proc helper {} { return ::ns::helper }
    oo::class create C { forward f helper ;# CALL
    }
}
puts [[::ns::C new] f]
