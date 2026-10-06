set rows {}
foreach expression {
    {"abcd" matches "a*"}
    {"a*" matches "a*"}
    {"a*" matches "abcd"}
    {"abcd" matches "a.*"}
    {"abcd" matches "bc"}
    {"abcd" matches "abcd"}
    {1 or 0 matches 0}
    {not "abc" starts_with "a"}
    {not ("abc" starts_with "a")}
} {
    set rc [catch {expr $expression} result]
    lappend rows [list $expression $rc $result]
}
set rows
