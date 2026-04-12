# Tests boolean vs integer shimmering
set flag "true"

for {set i 0} {$i < 10} {incr i} {
    # Force boolean intrep
    if {$flag} {
        # Force integer intrep
        incr flag 1
    }
}
