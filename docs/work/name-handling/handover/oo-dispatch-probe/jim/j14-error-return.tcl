# Jim error/return/catch differences
puts [catch {error msg {custom stack}} r opts]:[dict get $opts -errorinfo]
puts [catch {return -code 5 x} r]:$r
puts [catch -error {error x} r]:$r
puts [catch -break {break} r]:$r
proc lvl {} {return -level 2 deep}
proc mid {} {lvl; return mid}
puts [mid]
