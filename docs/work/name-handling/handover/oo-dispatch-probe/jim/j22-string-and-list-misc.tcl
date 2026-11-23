puts [catch {string byterange abcdef 1 3} r]:$r
puts [catch {lsearch -command {string equal} {a b} b} r]:$r
puts [catch {lassign {1 2 3} a b} r]:$r
puts [catch {lreverse {1 2 3}} r]:$r
