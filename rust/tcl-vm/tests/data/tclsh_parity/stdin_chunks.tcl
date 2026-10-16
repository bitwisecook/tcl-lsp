# tclsh reads its stdin a line at a time until the lines read form complete
# commands, evaluates them, prints a failure's message and reads on.
puts a
set x [list 1 \
    2]
puts $x; nosuch; puts unreached
puts b
set y {
1}
proc p {} {
    error inner
}
p
if 1 {
    puts c
}
puts [info complete "set z \{"]
foreach i {1 2} {
    puts $i
}
break
puts d
exit
puts never
