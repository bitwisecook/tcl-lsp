proc p {} {
    set ::y 789
    return $::y
}
set ::a(1) 2
set ::a($::a(1)) 3
