# T5: catch/error/return -code/errorCode - the completion spine.
proc risky {x} {
    if {$x < 0} { error "negative: $x" NEGATIVE {MYERR NEG} }
    return [expr {sqrt($x)}]
}
puts [catch {risky 4} r]
puts $r
puts [catch {risky -1} r opts]
puts $r
puts [dict get $opts -errorcode]
puts $::errorCode
proc early {} { return -code break }
puts [catch early r]
proc custom {} { return -code 5 done }
puts [catch custom r]
puts $r
