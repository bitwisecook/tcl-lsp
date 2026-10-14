if {[llength [info commands oo::class]] == 0} {
    puts "UNSUPPORTED TclOO"
    exit 0
}
namespace eval Outer {
    namespace eval Inner {
        oo::class create Base {method run {} {return BASE}}
        oo::class create Child {superclass Base; method run {} {return CHILD}}
        oo::class create Grandchild {superclass Child; method run {} {return GRANDCHILD}}
        puts "SUPER [info class superclasses Child] [info class superclasses Grandchild]"
    }
}
namespace eval Traits {
    namespace eval Inner {
        oo::class create Base {method run {} {return BASE}}
        oo::class create Child {mixin Base; method run {} {return CHILD}}
        oo::class create Grandchild {superclass Child; method run {} {return GRANDCHILD}}
        puts "MIXIN [info class mixins Child] [info class superclasses Grandchild]"
    }
}
namespace eval A {oo::class create Base {method run {} {return A}}}
namespace eval B {
    oo::class create Base {method run {} {return B}}
    oo::class create Child {superclass Base; method run {} {return CHILD}}
    puts "OWNER [info class superclasses Child]"
}
oo::class create 茶 {method run {} {return ROOT}}
set colonCode [catch {
    namespace eval : {
        oo::class create 茶 {method run {} {return COLON}}
        oo::class create Leaf {superclass 茶}
        puts "COLON [info class superclasses Leaf]"
    }
} colonMessage]
puts "COLON_PREFLIGHT $colonCode $colonMessage"
puts "ROOT [[茶 new] run]"
