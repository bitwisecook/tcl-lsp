proc get_color {level} {
    set _map [dict create "error" red "warning" yellow "info" blue "debug" grey]
    set color [dict get $_map $level]
    return $color
}
