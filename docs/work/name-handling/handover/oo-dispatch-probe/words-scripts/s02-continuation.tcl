set a [list x \
    y]
set b {x \
    y}
set c "x \
    y"
puts [list [llength $a] [string length $b] $c]
