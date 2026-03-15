# Calculate the sum of a list of numbers
proc sum_list {numbers} {
    set total 0
    foreach num $numbers {
        # Accumulate each number
        set total [expr {$total + $num}]
    }
    return $total
}

# Find the maximum value in a list
proc find_max {lst} {
    set max [lindex $lst 0]
    for {set i 1} {$i < [llength $lst]} {incr i} {
        set val [lindex $lst $i]
        if {$val > $max} {
            set max $val
        }
    }
    return $max
}

# Main script
set data {3 1 4 1 5 9 2 6}

# Process the data
set total [sum_list $data]
set biggest [find_max $data]

puts "Sum: $total"
puts "Max: $biggest"
