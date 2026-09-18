proc get_color {level} {
    set color_map [dict create "error" red "warning" yellow "info" blue "debug" grey]
    set color [dict get $color_map ${level}]
    return $color
}
