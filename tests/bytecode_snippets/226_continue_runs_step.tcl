# ``continue`` inside a ``for`` loop must still run the ``next_script``
# (step) on its way back to the header, exactly like reference Tcl.
# Previously the CFG-based emitter wrapped body+step in a single inner
# block and ``continue`` br'd out past BOTH, so the loop variable was
# never advanced: ``for {set i 0} {$i<5} {incr i} {continue}`` spun
# forever on ``$i == 0``.
set hit {}
for {set i 0} {$i < 5} {incr i} {
    if {$i == 2} continue
    lappend hit $i
}
if {$hit ne {0 1 3 4}} { error "for continue broke: $hit" }

# Continue in a while loop still skips the rest of the body (no step
# block exists), so the regression check above must not regress while.
set i 0; set j 0
while {$i < 5} {
    incr i
    if {$i == 3} continue
    incr j
}
if {$j != 4} { error "while continue broke: $j" }

# break exits the loop entirely.
set broke_at -1
for {set k 0} {$k < 10} {incr k} {
    if {$k == 3} { set broke_at $k; break }
}
if {$broke_at != 3} { error "break broke" }
return ok
