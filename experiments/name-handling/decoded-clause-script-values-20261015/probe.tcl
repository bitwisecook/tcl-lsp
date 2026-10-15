puts [list PATCHLEVEL [info patchlevel]]
foreach {label script} {
    IF_LITERAL {if {1} {{return one}}}
    IF_RETURN {if {1} {return one}}
    ELSEIF_LITERAL {if {0} {} elseif {1} {{return one}}}
    ELSE_LITERAL {if {0} {} else {{return one}}}
} {
    set code [catch {eval $script} result]
    puts [list $label $code $result]
}
