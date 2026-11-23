# Trace callbacks run one level below the frame where the access happened.
proc cb {n1 n2 op} {
    puts "level=[info level] caller=[uplevel 1 {info level}] n1=$n1"
    upvar 1 $n1 v
    puts "seen=$v"
}
proc p {} { set loc 5; trace add variable loc write cb; set loc 6 }
p
set g 1
trace add variable g write cb
set g 2
proc q {} { upvar #0 g alias; set alias 3 }
q
