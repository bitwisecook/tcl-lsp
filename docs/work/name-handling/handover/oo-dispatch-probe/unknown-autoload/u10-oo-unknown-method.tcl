# TclOO: an unknown method receives unresolved method calls.
oo::class create C {
  method known {} { return known }
  method unknown {name args} { return "unknown method $name: $args" }
}
set o [C new]
puts [$o known]
puts [$o missing 1 2]
oo::class create D
puts "no unknown: [catch {[D new] missing} m] $m"
