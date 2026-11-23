puts [catch {dict getwithdefault {a 1} b 9} r]:$r
puts [catch {dict getdef {a 1} a 9} r]:$r
