# TIP 629/746: lseq
puts [catch {lseq 4} r]:$r
puts [catch {lseq 1 to 10 by 3} r]:$r
puts [catch {lseq 5 count 3} r]:$r
puts [catch {lseq 0 {1+2}} r]:$r
