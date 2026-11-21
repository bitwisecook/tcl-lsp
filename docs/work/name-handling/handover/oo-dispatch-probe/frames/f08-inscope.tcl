proc ::helper {} { return ::helper }
namespace eval ::lib { proc helper {} { return ::lib::helper } }
puts [namespace inscope ::lib { helper ;# CALL
}]
