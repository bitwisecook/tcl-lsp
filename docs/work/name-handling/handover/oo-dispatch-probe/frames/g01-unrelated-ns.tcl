namespace eval ::a { proc helper {} { return ::a::helper } }
namespace eval ::b {
    proc run {} { helper ;# CALL
    }
}
puts [catch ::b::run m]:$m
