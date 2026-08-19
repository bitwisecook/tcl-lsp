# A4: #1438 (no rollback) x #1440 (newest-first) interactions.

# 1. A write trace that mutates the same variable AND errors.
proc mut_boom {n1 n2 op} { set ::v 999 ; error boom }
set v old
trace add variable v write mut_boom
puts "1: [catch {set v new} m]:$m / v=$v"

# 2. Newest-first chain: the newest errors, the older ones must not fire.
set ::log {}
proc t1 args { lappend ::log 1 }
proc t2 args { lappend ::log 2 }
proc t3 args { lappend ::log 3; error stop }
set w old
trace add variable w write t1
trace add variable w write t2
trace add variable w write t3
puts "2: [catch {set w new} m]:$m / w=$w / log=$::log"

# 3. Newest mutates, oldest errors: which value survives?
set ::log {}
proc m1 args { lappend ::log m1; set ::y 111 }
proc m2 args { lappend ::log m2; error late }
set y old
trace add variable y write m2
trace add variable y write m1
puts "3: [catch {set y new} m]:$m / y=$y / log=$::log"

# 4. Array trace errors before the element trace: element trace must not fire.
set ::log {}
proc W args { lappend ::log W; error arrfail }
proc E args { lappend ::log E }
array set a {}
trace add variable a(k) write E
trace add variable a write W
puts "4: [catch {set a(k) 1} m]:$m / exists=[info exists a(k)] / val=[expr {[info exists a(k)] ? $a(k) : {-}}] / log=$::log"

# 5. Element trace errors; the array trace already ran and mutated.
set ::log {}
proc W2 args { lappend ::log W2; set ::b(other) touched }
proc E2 args { lappend ::log E2; error elemfail }
array set b {}
trace add variable b(k) write E2
trace add variable b write W2
puts "5: [catch {set b(k) 1} m]:$m / k=[expr {[info exists b(k)] ? $b(k) : {-}}] / other=[expr {[info exists b(other)] ? $b(other) : {-}}] / log=$::log"

# 6. A write trace that unsets its own variable, then errors.
proc kill {n1 n2 op} { unset ::z ; error gone }
set z old
trace add variable z write kill
puts "6: [catch {set z new} m]:$m / exists=[info exists z] / traces=[trace info variable z]"

# 7. incr/append/lappend with the erroring-and-mutating trace.
proc mut2 {n1 n2 op} { set ::n 777 ; error boom2 }
set n 5
trace add variable n write mut2
puts "7: [catch {incr n} m]:$m / n=$n"

# 8. A read trace that errors, newest-first, and the older read trace.
set ::log {}
proc r1 args { lappend ::log r1 }
proc r2 args { lappend ::log r2; error readfail }
set q val
trace add variable q read r1
trace add variable q read r2
puts "8: [catch {set copy $q} m]:$m / log=$::log"

# 9. Unset traces: errors ignored, all still fire, newest-first.
set ::log {}
proc u1 args { lappend ::log u1; error ignored1 }
proc u2 args { lappend ::log u2; error ignored2 }
set uu 1
trace add variable uu unset u1
trace add variable uu unset u2
puts "9: [catch {unset uu} m]:$m / exists=[info exists uu] / log=$::log"
