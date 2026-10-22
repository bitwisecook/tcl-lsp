namespace eval A {oo::class create Base {}}
namespace eval B {oo::class create Base {}; oo::class create Child {}}
namespace eval A {oo::define ::B::Child superclass Base}
puts [info class superclasses ::B::Child]
