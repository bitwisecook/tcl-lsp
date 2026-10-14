set delta 3
foreach program {{incr n $delta} {incr n {$delta}} {incr n "$delta"} {incr n 0\x33} {incr n {0\x33}}} {
    set n 5
    set code [catch $program value]
    puts [list amount $program $code $n $value]
}
set {cash$name} 4
set n 5
puts [list literal-sigil-read [incr n ${cash$name}]]
set {} 6
set n 5
puts [list empty-scalar-read [incr n ${}]]
set payload a\x62
puts [list escaped-value $payload]
set payload {a\x62}
puts [list braced-value $payload]
set payload {cash$name}
puts [list literal-sigil-value $payload]
