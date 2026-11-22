proc p {} {
    set a 01
    set b 0x10
    set c 1.0
    return [list $a [expr {$a + 0}] $b [expr {$b + 0}] $c [expr {$c == 1}] [string is integer $b] [expr {"1" eq "1.0"}]]
}
puts [p]
