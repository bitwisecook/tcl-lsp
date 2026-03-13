# Procedures and control flow
proc fib {n} {
    if {$n <= 1} {
        return $n
    }
    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
}

proc factorial {n} {
    set result 1
    for {set i 1} {$i <= $n} {incr i} {
        set result [expr {$result * $i}]
    }
    return $result
}

puts "fib(10) = [fib 10]"
puts "10! = [factorial 10]"

# While loop with break/continue
set i 0
while {$i < 20} {
    incr i
    if {$i == 5} { continue }
    if {$i == 15} { break }
    puts "i = $i"
}
