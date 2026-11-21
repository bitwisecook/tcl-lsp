proc ::helper {} { return ::helper }
namespace eval ::a {
    proc helper {} { return ::a::helper }
    proc run {} { helper ;# CALL
    }
}
puts [::a::run]
