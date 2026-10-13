puts [list IDENTITY [info patchlevel] [info library]]
foreach {label source} {
    bare {namespace which g}
    command {namespace which -command g}
    command-short {namespace which -c g}
    command-prefix {namespace which -comman g}
    variable {namespace which -variable g}
    variable-short {namespace which -v g}
    command-long {namespace which -commandx g}
    single-dash {namespace which - g}
    wrong-option {namespace which -unknown g}
    too-few {namespace which}
    too-many {namespace which -command g extra}
    dynamic-option {namespace which $option g}
    dynamic-name {namespace which -command $name}
    escaped-command {namespace which -co\mmand g}
    quoted-command {namespace which "-command" g}
    braced-command {namespace which {-command} g}
} {
    set code [catch {::tcl::unsupported::disassemble script $source} output]
    puts [list OBSERVATION $label $source $code $output]
}
