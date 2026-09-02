# T6: superclass, next, method resolution order, overriding.
oo::class create Animal {
    variable name
    constructor {n} { set name $n }
    method speak {} { return "$name makes a sound" }
    method name {} { return $name }
}
oo::class create Dog {
    superclass Animal
    constructor {n} { next $n }
    method speak {} { return "[next] (woof)" }
}
oo::class create Puppy {
    superclass Dog
    method speak {} { return "[next] tiny" }
}
set d [Dog new rex]
puts [$d speak]
set p [Puppy new bit]
puts [$p speak]
puts [$p name]
puts [info class superclasses Puppy]
puts [lsort [info class methods Dog -all]]
