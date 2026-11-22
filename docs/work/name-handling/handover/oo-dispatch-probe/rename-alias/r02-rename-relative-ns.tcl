namespace eval ::a { proc p {} {return ap}; rename p q }
puts [list [info commands ::a::*] [::a::q]]
namespace eval ::a { rename q ::r }
puts [list [info commands ::a::*] [::r]]
