set q 0
set r 0
set items [lindex $argv 0]
set c0 [lindex $argv 0]
set d0 [lindex $argv 0]
set e0 [lindex $argv 0]
set v0 [lindex $argv 0]
set c1 [lindex $argv 1]
set d1 [lindex $argv 1]
set e1 [lindex $argv 1]
set v1 [lindex $argv 1]
set c2 [lindex $argv 2]
set d2 [lindex $argv 2]
set e2 [lindex $argv 2]
set v2 [lindex $argv 2]
set c3 [lindex $argv 3]
set d3 [lindex $argv 3]
set e3 [lindex $argv 3]
set v3 [lindex $argv 3]
set c4 [lindex $argv 4]
set d4 [lindex $argv 4]
set e4 [lindex $argv 4]
set v4 [lindex $argv 4]
set c5 [lindex $argv 5]
set d5 [lindex $argv 5]
set e5 [lindex $argv 5]
set v5 [lindex $argv 5]
set c6 [lindex $argv 6]
set d6 [lindex $argv 6]
set e6 [lindex $argv 6]
set v6 [lindex $argv 6]
set c7 [lindex $argv 7]
set d7 [lindex $argv 7]
set e7 [lindex $argv 7]
set v7 [lindex $argv 7]
set c8 [lindex $argv 8]
set d8 [lindex $argv 8]
set e8 [lindex $argv 8]
set v8 [lindex $argv 8]
set c9 [lindex $argv 9]
set d9 [lindex $argv 9]
set e9 [lindex $argv 9]
set v9 [lindex $argv 9]
set c10 [lindex $argv 10]
set d10 [lindex $argv 10]
set e10 [lindex $argv 10]
set v10 [lindex $argv 10]
set c11 [lindex $argv 11]
set d11 [lindex $argv 11]
set e11 [lindex $argv 11]
set v11 [lindex $argv 11]
set x 0
switch -- $v0 {
    a { set x 0 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v1 {
    a { set x 1 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v2 {
    a { set x 2 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v3 {
    a { set x 3 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v4 {
    a { set x 4 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v5 {
    a { set x 5 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v6 {
    a { set x 6 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v7 {
    a { set x 7 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v8 {
    a { set x 8 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v9 {
    a { set x 9 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v10 {
    a { set x 10 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
switch -- $v11 {
    a { set x 11 }
    b0 { }
    b1 { }
    b2 { }
    b3 { }
}
puts $x
