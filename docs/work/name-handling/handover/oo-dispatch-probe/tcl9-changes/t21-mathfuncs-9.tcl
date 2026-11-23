puts [catch {expr {isinf(1.0)}} r]:$r
puts [catch {expr {isnan(1.0)}} r]:$r
puts [catch {fpclassify 1.0} r]:$r
puts [catch {expr {int(2**70)}} r]:$r
puts [catch {expr {cbrt(27.0)}} r]:$r
puts [catch {expr {log2(8.0)}} r]:$r
