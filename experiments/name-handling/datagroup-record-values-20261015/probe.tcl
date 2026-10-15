puts [list PATCHLEVEL [info patchlevel]]
set value VALUE
foreach {label script} {
    QUOTED {return "80"}
    LITERAL_QUOTES {return {"80"}}
    LITERAL_SPACES {return { 80 }}
    LITERAL_SIGILS {return {$value[missing]}}
    ESCAPED_NUMERIC {return \u0038\u0030}
    VARIABLE {return $value}
    COMMAND {return [list computed]}
} {
    set code [catch {eval $script} result]
    puts [list $label $code [string length $result] $result]
}
