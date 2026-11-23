# interp create reads options anywhere before --, by unique prefix (Tcl_GetIndexFromObj).
interp create -s b
interp create c -safe
interp create -- -safe
puts "b=[interp issafe b] c=[interp issafe c] dash=[interp issafe -safe]"
puts "b-exec=[catch {interp eval b {exec echo hi}} m] $m"
puts "bogus=[catch {interp create d -bogus} m] $m exists=[interp exists d]"
