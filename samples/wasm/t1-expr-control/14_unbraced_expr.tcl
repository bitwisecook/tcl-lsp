# T1: unbraced expr is double-substituted; this must NOT be native-compiled
# from the literal text, but the compiler can prove the substituted text is
# a constant expression here and fold it.
set x 4
set op +
puts [expr $x $op 1]
puts [expr "$x * 2"]
