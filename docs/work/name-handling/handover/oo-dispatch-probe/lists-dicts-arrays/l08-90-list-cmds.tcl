set l {a b c d}
set m {a b c d}
ledit m 1 2 X Y Z
puts [list [lseq 1 5] [lseq 0 10 3] [lseq 5 1] [lpop l] $l [lremove {a b c d} 1 2] $m [lseq 1 count 3]]
