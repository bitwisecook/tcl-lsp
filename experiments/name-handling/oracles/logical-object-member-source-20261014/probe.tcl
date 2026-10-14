if {[llength [info commands ::oo::class]] == 0} {
    puts TCL_OO_UNAVAILABLE
    exit 0
}
::oo::class create C {method base {} {return BASE}}
C create {$literal}
C create literal
::oo::objdefine {$literal} {method own {} {return LITERAL}}
::oo::objdefine literal {method own {} {return BARE}}
set o [C new]
::oo::objdefine "$o" {method {two words} {} {return SPACE}; unexport base}
puts [list literal [{$literal} own] bare [literal own] quoted [$o {two words}]]
puts [list masked [catch {$o base} message] own [$o {two words}]]
