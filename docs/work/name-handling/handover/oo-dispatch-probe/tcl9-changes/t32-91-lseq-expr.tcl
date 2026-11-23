# TIP 746: lseq no longer evaluates expressions (9.0.4 backport, 9.1)
puts [catch {lseq 1 {2+1}} r]:$r
