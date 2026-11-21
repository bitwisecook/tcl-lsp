foreach n {::a::b::c a::b ::a :: a::::b a:::b :::a a:: ::a:: {} a} { lappend r "[list $n]=[list [namespace qualifiers $n]]|[list [namespace tail $n]]" }
puts [join $r " "]
