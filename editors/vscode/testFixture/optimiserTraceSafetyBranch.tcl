proc onread {name1 name2 op} {
    puts "trace fired"
}
proc setup {} {
    trace add variable ::x read onread
}
set x 1
setup
if {$x} {
    puts yes
} else {
    puts no
}
