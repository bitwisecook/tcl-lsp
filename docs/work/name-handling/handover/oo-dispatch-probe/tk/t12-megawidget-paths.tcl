package require Tk
proc mkdialog {w} {
    toplevel $w
    button $w.ok -text OK -command [list destroy $w]
    pack $w.ok
    return $w
}
set d [mkdialog .dlg]
$d.ok configure -text Done
puts [$d.ok cget -text]
puts [catch {$d.ok configure -txt x} m]:$m ;# BAD option via built path
