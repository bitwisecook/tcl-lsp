set l {a b c d}
puts [list [lrange $l 1 end-1] [lrange $l 3 1] [lrange $l -5 1] [lreplace $l 1 2 X] [lreplace $l 1 0 Y] [lreplace $l end end] [catch {lreplace $l 9 9 Z} m] $m [lreplace $l 5 5]]
