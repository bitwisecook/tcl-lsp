proc onread {name1 name2 op} {
    puts "trace fired"
}
proc setup {} {
    trace add variable ::x read onread
}
setup
set x 5
puts $x
