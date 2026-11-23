# interp and child-object subcommands accept unique prefixes (Tcl_GetIndexFromObj).
set c [interp cr]
puts "create-prefix=[interp exi $c]"
puts "ambiguous=[catch {interp ex $c} m] $m"
puts "child-prefix=[$c ev {expr {2 + 2}}]"
puts "cancel=[catch {interp cancel $c} m] <$m>"
puts "bad=[catch {interp bogus} m] $m"
interp del $c
