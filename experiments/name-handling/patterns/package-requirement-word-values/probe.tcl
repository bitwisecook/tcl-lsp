foreach script {
    {package vsatisfies 8.6 8.6}
    {package vsatisfies 8.6 {8.6}}
    {package vsatisfies 8.6 "8.6"}
    {package vsatisfies 8.6 8\u002e6}
    {package vsatisfies 8.6 {{8.6}}}
    {package vsatisfies 8.6 {"8.6"}}
    {package vsatisfies 8.6 \"8.6\"}
    {package vsatisfies 8.6 $unavailable_requirement}
} {
    set code [catch {eval $script} result]
    puts [list $script $code $result]
}
