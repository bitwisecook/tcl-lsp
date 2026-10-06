proc show {label expression} {
    set code [catch {uplevel 1 [list expr $expression]} value]
    set representation unavailable
    catch {set representation [::tcl::unsupported::representation $value]}
    puts [list $label $code $value $representation]
}
set base 2
set exponent -1
set floating 2.0
show pow-negative {$base ** $exponent}
show pow-positive {$base ** 3}
show mod-integer {$base % 3}
show mod-floating {$floating % 3}
show divide-integer {$base / 3}
show divide-floating {$floating / 3}
proc pooled {} {return [expr {7 << 1}]}
set held [pooled]
llength $held
set value [pooled]
set representation unavailable
catch {set representation [::tcl::unsupported::representation $value]}
puts [list pool-after-list $value $representation]
