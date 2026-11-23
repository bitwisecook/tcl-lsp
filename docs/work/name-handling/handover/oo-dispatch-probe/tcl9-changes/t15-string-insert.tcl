puts [catch {string insert abc 1 X} r]:$r
puts [catch {string insert abc end-1 X} r]:$r
puts [catch {string is dict {a b c}} r]:$r
