set pos 0
set re {\{([A-Za-z0-9_]+)\|([^{}]*)\}}
set line "line"

while {[regexp -start $pos -indices $re $line all nameIdx textIdx]} {
    lassign $all start end
    if {$start > $pos} {
        # do something
    }
}
