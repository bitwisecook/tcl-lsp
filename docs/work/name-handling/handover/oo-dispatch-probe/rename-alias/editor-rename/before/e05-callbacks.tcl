proc helper {args} {lappend ::log $args}
after 0 helper a
set cb [list helper b]
{*}$cb
trace add variable v write [list helper c]
set v 1
update
puts [llength $::log]
