proc mk {name supers} {
    oo::class create $name
    if {[llength $supers]} { oo::define $name superclass {*}$supers }
    oo::define $name method who {} [string map [list NAME $name] {
        set r NAME
        if {![catch {next} rest]} { append r " " $rest }
        return $r
    }]
}
mk X {}; mk Y {}
mk A {X Y}; mk B {Y X}
mk D {A B}
puts "inconsistent: [[D new] who]"
mk P {}; mk Q {P}; mk R {P}; mk S {Q}; mk T {R S}
puts "deep: [[T new] who]"
mk K {}; mk L {K}; mk M {}; mk N {L M K}
puts "explicit-base: [[N new] who]"
