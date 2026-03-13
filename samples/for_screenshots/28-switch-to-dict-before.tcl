proc get_color {level} {
    switch -exact -- $level {
    #   ^--- cursor on switch
        "error"   { set color red }
        "warning" { set color yellow }
        "info"    { set color blue }
        "debug"   { set color grey }
    }
    return $color
}
