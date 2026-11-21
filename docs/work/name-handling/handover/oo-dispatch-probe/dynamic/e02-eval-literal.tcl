proc p {} {
    eval {set y 2}
    eval [list set z 3]
    return $y$z
}
puts [p]
proc q {} { return [catch {eval {set w [string lenght abc]}} m]:$m } ;# BAD typo in literal eval
puts [q]
