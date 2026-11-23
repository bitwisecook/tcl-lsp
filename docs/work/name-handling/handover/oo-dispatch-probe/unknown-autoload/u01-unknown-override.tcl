# A user unknown procedure receives the command words of an unresolved call.
rename unknown _orig_unknown
proc unknown {args} { return "unknown got: $args" }
puts [nosuch a {b c}]
puts [::ns::deep x]
rename unknown ""
puts "no unknown: [catch {nosuch2 y} m] $m"
