proc destroy {w} { return "destroying $w" }
proc RestoreFocusGrab {grab focus {destroy destroy}} {
    if {$destroy eq "destroy"} { return [destroy $grab] }
    return $destroy
}
