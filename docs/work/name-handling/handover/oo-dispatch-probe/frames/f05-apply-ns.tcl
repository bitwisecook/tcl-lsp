proc ::helper {} { return ::helper }
namespace eval ::lib { proc helper {} { return ::lib::helper } }
puts [apply {{} { helper ;# CALL
} ::lib}]
