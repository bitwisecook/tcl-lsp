# A3: op-list / letter-string parse edges, and the option word itself.
proc cb args {}
proc show {label script} {
    puts "$label: [catch {uplevel 1 $script} m]:$m"
}
# Empty and whitespace-only op lists (modern).
show m1 {trace add variable v {} cb}
show m2 {trace add variable v {   } cb}
show m3 {trace remove variable v {} cb}
show m4 {trace add command cb {} cb}
show m5 {trace add execution cb {} cb}
# Abbreviations are rejected for operations (TCL_EXACT).
show m6 {trace add variable v w cb}
show m7 {trace add variable v {read wri} cb}
# Malformed list.
show m8 "trace add variable v \{ cb"
show m9 "trace add variable v \"a b\\\{\" cb"
# Empty and odd legacy letter strings.
show l1 {trace variable v {} cb}
show l2 {trace vdelete v {} cb}
show l3 {trace variable v { } cb}
show l4 {trace variable v rwuax cb}
show l5 {trace variable v RW cb}
show l6 {trace variable v aaaa cb}
puts "l6i: [trace vinfo v] / [trace info variable v]"
# The option word.
show o1 {trace {} x}
show o2 {trace v x}
show o3 {trace vd x y z}
show o4 {trace vi x}
show o5 {trace a}
show o6 {trace r}
show o7 {trace re}
show o8 {trace Add variable v read cb}
# The type word.
show t1 {trace add {} v read cb}
show t2 {trace add v v read cb}
show t3 {trace add e cb enter cb}
show t4 {trace add c cb delete cb}
