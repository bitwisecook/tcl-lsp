# Issue #923 idx 48 — a query anchored on a bare declaring token (a `set`
# left-hand side, a `foreach` loop variable) must answer exactly as a query
# anchored on a later `$name` read of the same cell.
set idx48Total 10
puts $idx48Total
puts $idx48Total
foreach idx48Cmd {alpha beta} {
    puts "cmd is $idx48Cmd"
}
