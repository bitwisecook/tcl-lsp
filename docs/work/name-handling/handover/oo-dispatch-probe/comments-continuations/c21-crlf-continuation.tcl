set a 1\
puts [catch {eval {set b x\
}} r]:[string length $r]
puts [string length "p\
q"]
