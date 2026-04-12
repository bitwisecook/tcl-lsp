set result {}
foreach {a b} {1 2 3 4} c {x y z} {
    lappend result "$a$b$c"
}
