foreach script [list {if 1 "\u0072eturn"; set after SURVIVED} {if 1 {set kept {{literal}}}; set after SURVIVED} {if 1 "$missing"; set after SURVIVED}] {
    set code [catch {eval $script} result]
    puts [list CASE $script $code $result]
}
