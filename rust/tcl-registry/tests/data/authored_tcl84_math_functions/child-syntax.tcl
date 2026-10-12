set malformed "abs(\[set x \{\])"
set code [catch {expr $malformed} value]
puts [list malformed-argument $code $value]
set malformed "pow(\[set x \{\])"
set code [catch {expr $malformed} value]
puts [list malformed-required-argument $code $value]
set malformed "future(\[set x \{\])"
set code [catch {expr $malformed} value]
puts [list unknown-before-malformed $code $value]
