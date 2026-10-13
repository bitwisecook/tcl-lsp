set ::observations {}
foreach script {
    {apply {{} {array set a {}; list [array size a] [lsort [array get a]]}}}
    {apply {{} {array set a {a 1}; list [array size a] [lsort [array get a]]}}}
    {apply {{} {array set a {a 1 b 2}; list [array size a] [lsort [array get a]]}}}
    {apply {{} {array set a {a 1 b 2 c 3}; list [array size a] [lsort [array get a]]}}}
    {apply {{} {array set a {aa 11 bb 22 cc 33 dd 44 ee 55 ff 66}; list [array size a] [lsort [array get a]]}}}
    {apply {{} {array set a {a 1 b 2 c 3 d 4 e 5 f 6 g 7 h 8}; list [array size a] [lsort [array get a]]}}}
} {
    lappend ::observations [uplevel #0 $script]
}
set ::observations
