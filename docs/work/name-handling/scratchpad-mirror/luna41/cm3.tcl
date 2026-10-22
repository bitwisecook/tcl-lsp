oo::class create F {
    classmethod m {} { return delegate }
    self method m {} { return self }
}
puts "F m -> [F m]"
oo::class create G { superclass F }
oo::objdefine G method m {} { return Gself }
puts "G m -> [G m]"
oo::class create H { self method own {} { return own } }
oo::class create K { superclass H }
puts "K own -> [catch {K own} e] $e"
puts "info: [info object mixins F] [info object mixins G]"
