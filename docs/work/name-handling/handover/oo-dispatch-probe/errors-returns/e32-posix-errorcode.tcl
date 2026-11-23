# Filesystem failures set errorCode to POSIX <errno> <message>, which `trap {POSIX ENOENT}` matches.
catch {open /no/such/file} m; puts "open: $::errorCode"
catch {cd /no/such/dir} m; puts "cd: $::errorCode"
catch {file stat /no/such/file st} m; puts "stat: $::errorCode"
puts "trap: [catch {try { open /no/such/file } trap {POSIX ENOENT} {} { puts trapped }} r] $r"
