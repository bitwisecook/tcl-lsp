proc ::helper {} { return ::helper }
namespace eval ::a {
    proc helper {} { return ::a::helper }
    proc run {} { uplevel #0 { helper ;# CALL
    } }
}
puts [::a::run]
