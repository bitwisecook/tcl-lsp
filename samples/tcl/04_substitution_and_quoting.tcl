# 04: Variable substitution, quoting, braces, and command substitution

set arr(user) "jimd"
set who $arr(user)

set expanded "user=[string toupper $who]"
set literal {$who is not substituted here}

puts $expanded
puts $literal
puts [format "time=%s" [clock format [clock seconds] -format %H:%M:%S]]
