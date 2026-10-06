set left "A\x00x"
set right "A\x00y"
puts [list equal [string equal $left $right]]
puts [list length [string length $left]]
puts [list listlength [llength {a b}]]
puts [list equalnocase [string equal -nocase A a]]
puts [list llengthempty [catch {llength} message] $message]
puts [list llengthextra [catch {llength a b} message] $message]
puts [list lengthmissing [catch {string length} message] $message]
puts [list equalmissing [catch {string equal a} message] $message]
