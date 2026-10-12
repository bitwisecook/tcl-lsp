proc observe {label body} {
    set ::before 0
    proc compiled {} $body
    set code [catch compiled result]
    puts [list $label $code $::before [string map [list "\n" "\\n"] $result]]
}
observe if-zero {set ::before 1; if {0} {set x extra bad}; return OK}
observe if-zero-float {set ::before 1; if {0.0} {set x extra bad}; return OK}
observe if-zero-hex {set ::before 1; if {0x0} {set x extra bad}; return OK}
observe if-off {set ::before 1; if {off} {set x extra bad}; return OK}
observe if-expression-zero {set ::before 1; if {0+0} {set x extra bad}; return OK}
observe if-variable-zero {set ::before 1; set flag 0; if {$flag} {set x extra bad}; return OK}
observe if-dead-else {set ::before 1; if {1} {set x yes} else {set x extra bad}; return OK}
observe if-dead-elseif {set ::before 1; if {1} {set x yes} elseif {$flag} {set x extra bad}; return OK}
observe while-zero {set ::before 1; while {0} {set x extra bad}; return OK}
observe while-expression-zero {set ::before 1; while {0+0} {set x extra bad}; return OK}
observe for-zero {set ::before 1; for {} {0} {} {set x extra bad}; return OK}
observe foreach-empty {set ::before 1; foreach x {} {set x extra bad}; return OK}
observe protected-catch {set ::before 1; catch {set x extra bad} result; return $result}
observe return-before-error {set ::before 1; return OK; set x extra bad}
observe expr-premature {set ::before 1; expr {+}}
observe expr-multiple-arguments {set ::before 1; expr + +}
observe expr-substituted {set ::before 1; set expression +; expr $expression}
observe if-premature {set ::before 1; if {+} {set x yes}}
observe while-premature {set ::before 1; while {+} {set x yes}}
observe for-premature {set ::before 1; for {} {+} {} {set x yes}}
observe if-error-order {set ::before 1; if {+} {set x extra bad}}
observe while-error-order {set ::before 1; while {+} {set x extra bad}}
observe for-error-order {set ::before 1; for {} {+} {} {set x extra bad}}
observe catch-expression-error {set ::before 1; catch {expr {+}} result; return $result}
observe command {set ::before 1; expr {[set x extra bad]}}
observe short-and {set ::before 1; expr {0 && [set x extra bad]}}
observe short-or {set ::before 1; expr {1 || [set x extra bad]}}
observe conditional {set ::before 1; expr {0 ? [set x extra bad] : 1}}
observe array-index {set ::before 1; set x $a([incr bad extra args])}
observe for-initial {set ::before 1; for [incr bad extra args] {0} {} {set x extra bad}}
observe math-unknown-syntax {set ::before 1; expr {future_function(1+)}}
observe math-unknown-trailing {set ::before 1; expr {future_function(1,)}}
observe math-unknown-first {set ::before 1; expr {future_function([set x extra bad])}}
observe math-required-extra {set ::before 1; expr {abs([set x extra bad],2)}}
observe math-required-missing {set ::before 1; expr {pow([set x extra bad])}}
observe math-nested-unknown {set ::before 1; expr {abs(unknown(1),2)}}
observe math-extra-not-visited {set ::before 1; expr {abs(1,[set x extra bad])}}
observe math-second-required {set ::before 1; expr {pow(1,[set x extra bad],3)}}
observe error-message-before-argv {set ::before 1;error [rename error native_error;proc error args {return CUSTOM};list MESSAGE]}
rename error {}
rename native_error error
observe error-info-before-argv {set ::before 1;error MESSAGE [rename error native_error;proc error args {return CUSTOM};list INFO]}
rename error {}
rename native_error error
rename set native_set
proc renamed_compiled {} {native_set x extra bad}
puts [list renamed-set-error [catch renamed_compiled result] $result]
