# Since 8.5 a wrong-args error through an alias names the alias, not the target.
proc target {} { return ok }
interp alias {} viaAlias {} target
puts "same-interp=[catch {viaAlias extra} m] $m"
set c [interp create]
interp alias $c childAlias {} target
puts "child-to-parent=[catch {interp eval $c {childAlias extra}} m] $m"
interp eval $c { proc ctarget {} { return ok } }
interp alias {} parentAlias $c ctarget
puts "parent-to-child=[catch {parentAlias extra} m] $m"
interp delete $c
