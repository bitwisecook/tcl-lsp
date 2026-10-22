# Finite pinned Jim semantics; does not measure analyser behavior.
proc vm_expect {label actual expected} {
    if {![string equal $actual $expected]} {
        error [list mismatch $label actual $actual expected $expected]
    }
    puts [list PASS $label $actual]
}
puts [list version [info patchlevel]]
vm_expect plain-definition-result [proc vm_plain {a b} {return $a$b}] vm_plain
vm_expect plain-arguments [info args vm_plain] {a b}
vm_expect plain-call [vm_plain x y] xy
vm_expect statics-definition-result [proc vm_counter {} {{n 0}} {incr n}] vm_counter
vm_expect statics-first [vm_counter] 1
vm_expect statics-second [vm_counter] 2
vm_expect statics-not-formals [info args vm_counter] {}
set seed 5
set shared 1
proc vm_mix {a} {seed {n 0} &shared} {
    incr n
    return [list $a $seed $n $shared]
}
set seed 6
set shared 2
vm_expect static-copy-literal-reference [vm_mix 9] {9 5 1 2}
vm_expect static-next-call [vm_mix 10] {10 5 2 2}
vm_expect static-no-extra-arity [catch {vm_mix} message] 1
vm_expect static-malformed-three-fields [catch {proc vm_invalid {} {{a b c}} {return 1}} message] 1
vm_expect proc-too-few [catch {proc vm_short {}} message] 1
vm_expect proc-too-many [catch {proc vm_long {} {} {} {}} message] 1
proc {vm two-word} {x} {return $x}
vm_expect two-word-command [{vm two-word} 7] 7
vm_expect other-two-word-unknown [catch {{vm absent} 7} message] 1
class vmPoint {x 0 y 0}
class vmPoint3 vmPoint {z 0}
set point [vmPoint new {x 3 y 4}]
set point3 [vmPoint3 new]
vm_expect class-dictionary [$point get x] 3
vm_expect inherited-variable [$point3 get x] 0
vm_expect own-derived-variable [$point3 get z] 0
vmPoint method norm {} {return [expr {abs($x) + abs($y)}]}
vm_expect method-instance-variables [$point norm] 7
proc {vmPoint raw} {dx} {return $dx}
vm_expect raw-two-word-method [$point raw 9] 9
proc {vmPoint rawbad} {} {return $x}
vm_expect raw-method-lacks-instance-binding [catch {$point rawbad} message] 1
vm_expect odd-variable-dictionary [catch {class vmOdd {x 0 y}} message] 1
vm_expect missing-variable-dictionary [catch {class vmLonely} message] 1
vm_expect loop-present [expr {[llength [info commands loop]] == 1}] 1
vm_expect sleep-present [expr {[llength [info commands sleep]] == 1}] 1
puts [list completed controls 25]
