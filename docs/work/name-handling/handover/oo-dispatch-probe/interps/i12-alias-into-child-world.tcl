# Aliases resolve the target command in the target interpreter at call time.
set c [interp create]
interp alias {} viaChild $c compute
puts "missing=[catch {viaChild} m] $m"
interp eval $c { proc compute {} { return child-compute } }
puts "defined=[viaChild]"
proc compute {} { return parent-compute }
puts "still-child=[viaChild]"
interp delete $c
puts "after-delete=[catch {viaChild} m] $m"
