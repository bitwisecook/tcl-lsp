proc chain {o} { set r {}; foreach e [info object call $o m] { lappend r [lindex $e 2]:[lindex $e 0] }; return $r }
foreach c {A B C D X Y Z M F} { oo::class create $c { method m {} {} } }
# diamond
oo::define B superclass A
oo::define C superclass A
oo::define D superclass B C
puts "diamond D(B,C) B(A) C(A): [chain [D new]]"
# order conflict C3 rejects
oo::define X superclass A B
oo::define Y superclass B A
oo::define Z superclass X Y
puts "conflict Z(X,Y) X(A,B) Y(B,A): [chain [Z new]]"
# mixin on class and object, filter
oo::define D mixin M
set o [D new]
oo::objdefine $o mixin F
oo::objdefine $o method m {} {}
puts "mixins: [chain $o]"
oo::define D { method f args { next {*}$args }; filter f }
puts "filter: [lrange [info object call $o m] 0 1]"
