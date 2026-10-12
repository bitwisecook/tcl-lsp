set observed {}
test jim-lifecycle-1 {setup body cleanup retain variable continuity} -setup {
    set payload {a b}
} -body {
    lappend observed [set payload]
} -cleanup {
    lappend observed [set payload]
    unset payload
} -result {{a b}}
list $observed [info exists payload] $::testinfo(numpass) $::testinfo(numfail)
