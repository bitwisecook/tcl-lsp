proc ::helper {} { return ::helper }
namespace eval ::a {
    proc helper {} { return ::a::helper }
    interp alias {} ::a::al {} helper ;# CALL
}
puts [::a::al]
