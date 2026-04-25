# Source: tcltest lsetComp with end-N index
set lst {a b c d e}
lset lst end-1 X
puts $lst
