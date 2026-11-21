proc ::helper {} { return ::helper }
namespace eval ::a { proc helper {} { return ::a::helper } }
puts [namespace eval ::a { helper ;# CALL
}]
