proc forward {prefix} {eval $prefix}
proc change {name} {upvar 1 $name cell; set cell changed}
proc outer {} {
    set {a b} original
    forward [list change {a b}]
    set {a b}
}
outer

