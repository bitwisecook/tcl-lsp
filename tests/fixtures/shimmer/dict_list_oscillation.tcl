# Tests dict and list type thunking (oscillation) within a loop
set data {a 1 b 2 c 3}

for {set i 0} {$i < 10} {incr i} {
    # Force dict intrep
    dict get $data "a"
    
    # Force list intrep
    lindex $data 2
}
