proc forward {prefix} {uplevel 1 $prefix}
proc change {name} {upvar 1 $name cell; set cell changed}
proc outer {} {
    set {a b} original
    forward [list change {a b}]
    set {a b}
}
outer

