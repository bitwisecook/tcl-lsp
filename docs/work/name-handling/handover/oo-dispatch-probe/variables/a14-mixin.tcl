oo::class create M { method mm {} { return mixed } }
oo::class create C { mixin M }
puts [[C new] mm]
