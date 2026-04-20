# Source: tcltest lsetComp with multi-level index list
set lst {{1 2} {3 4} {5 6}}
lset lst 1 1 X
puts $lst
