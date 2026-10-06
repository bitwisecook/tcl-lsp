proc observe {name script} {
    set code [catch {uplevel 1 $script} result]
    puts [list $name $code $result]
}
observe formal21 {proc double {n} {expr {$n * 2}}; double 21}
observe formal22 {double 22}
observe fixed_with_rest {proc f {a args} {expr {$a * 2}}; f 5 x y z}
observe original_octal {proc f {a} {expr {$a * 2}}; f 005}
observe unrelated_rename {proc unrelated {} {return 99}; rename unrelated moved; double 21}
observe known_double {proc f {a} {expr {$a * 2}}; f [expr {double(1)}]}
observe invalid_number {f not_a_number}
observe division_error {proc divide {n} {expr {$n / 0}}; divide 21}
observe double_default {proc floating {} {expr {double(21)}}; floating}
set tcl_precision 1
observe double_precision_one {floating}
