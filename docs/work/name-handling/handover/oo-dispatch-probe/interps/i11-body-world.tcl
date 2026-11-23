# The body passed to interp eval runs in the CHILD's global frame, not the
# caller's procedure frame: a local of the calling proc is invisible there.
proc work {} {
  set local 10
  set c [interp create]
  set r [catch {interp eval $c {set local}} m]
  interp eval $c { set local 20 }
  set inner [interp eval $c {set local}]
  interp delete $c
  return "$r/$m/$local/$inner"
}
puts [work]
# A child calling a proc of the parent fails unless aliased.
proc helper {} { return helped }
set c [interp create]
puts "direct=[catch {$c eval helper} m] $m"
$c alias helper helper
puts "aliased=[$c eval helper]"
interp delete $c
