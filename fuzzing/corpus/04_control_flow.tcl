# Control flow
set x 0
for {set i 0} {$i < 5} {incr i} {
    incr x $i
}

set y 10
while {$y > 0} {
    incr y -3
}

if {$x > 5} {
    set result "big"
} elseif {$x > 0} {
    set result "small"
} else {
    set result "zero"
}

foreach item {alpha beta gamma} {
    append buf $item
}
