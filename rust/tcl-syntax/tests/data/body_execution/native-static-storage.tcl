proc probe {label script} {puts [list $label [catch $script result] $result]}
probe literal {proc p {} {{x FIRST}} {set x [string cat $x +]; set x}; list [p] [p]}
probe copy {set x OLD; proc p {} {x} {set x}; set x NEW; list [p] $x}
probe captured {set x OLD; proc p {} {&x} {set x NEW}; p; set x}
probe recreated {set x OLD; proc p {} {&x} {set x}; unset x; set x NEW; list [p] $x}
probe shadow {set x OUTER; proc p {x} {&x} {set x}; list [p PARAM] $x}
probe localshadow {proc p {} {{x STATIC}} {set x LOCAL; set x}; list [p] [p]}
probe afterframe {proc maker {} {set x KEEP; proc p {} {&x} {set x}}; maker; p}
probe afterrename {proc p {} {{x 0}} {incr x}; rename p q; list [q] [q]}
probe redefine {proc p {} {{x 0}} {incr x}; set first [p]; proc p {} {{x 10}} {incr x}; list $first [p]}
probe targetalias {set a OLD; upvar 0 a x; proc p {} {&x} {set x}; set b NEW; upvar 0 b x; list [p] $x}
probe arraycopy {set a(k) VALUE; proc p {} {a(k)} {set a(k)}}
probe arrayref {set a(k) VALUE; proc p {} {&a(k)} {set a(k)}}
probe missing {proc p {} {absent} {set absent}}
probe duplicate {set x X; proc p {} {x &x} {set x}}
probe unsetstatic {proc p {} {{x STATIC}} {unset x; set x NEW; set x}; list [p] [p]}
probe defaultstatic {set x OUTER; proc p {{x DEFAULT}} {&x} {set x}; list [p] $x}
probe copyarray {set a(k) VALUE; proc p {} {a} {array get a}; set a(k) NEW; list [p] [array get a]}
probe capturedarray {set a(k) OLD; proc p {} {&a} {set a(k)}; unset a; set a(k) NEW; list [p] $a(k)}
probe copyarrayelementunset {set a(k) V; proc p {} {a} {unset a(k);info exists a(k)};list [p] $a(k)}
probe capturedarrayelementunset {set a(k) V;proc p {} {&a} {unset a(k);info exists a(k)};list [p] [info exists a(k)]}
