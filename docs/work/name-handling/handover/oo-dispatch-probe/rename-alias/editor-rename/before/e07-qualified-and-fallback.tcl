proc helper {} {return g}
puts [::helper]
namespace eval ::a { puts [helper] }
