# Runtime check: a lone non-decimal literal in expr inside catch
puts [expr {0o10}]
puts [catch {expr {0o10}} r]:$r
puts [catch {expr {0x10}} r]:$r
