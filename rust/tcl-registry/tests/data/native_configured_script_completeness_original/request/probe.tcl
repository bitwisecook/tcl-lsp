# Fixed original String inputs; this does not execute the candidate scripts.
puts "VERSION\t[info patchlevel]"
set cases {
    empty {}
    ordinary {115 101 116 32 120 32 49}
    open_brace {115 101 116 32 120 32 123}
    brace_literal_bracket {115 101 116 32 120 32 123 91 125}
    brace_literal_quote {115 101 116 32 120 32 123 34 125}
    open_quote {115 101 116 32 120 32 34}
    quoted_open_substitution {115 101 116 32 120 32 34 91 115 101 116 32 121}
    closed_quote_tail {115 101 116 32 120 32 34 97 34 98 32 123}
    closed_brace_tail {115 101 116 32 120 32 123 97 125 98 32 123}
    open_command {115 101 116 32 120 32 91 115 101 116 32 121}
    closed_command {115 101 116 32 120 32 91 115 101 116 32 121 93}
    var_open {115 101 116 32 120 32 36 123 97}
    var_inner_open_first_close {115 101 116 32 120 32 36 123 97 123 125}
    var_inner_balanced {115 101 116 32 120 32 36 123 97 123 98 125 99 125}
    var_escaped_close {115 101 116 32 120 32 36 123 97 92 125}
    var_unicode_first_close {115 101 116 32 120 32 36 123 233 123 125}
    quoted_var_first_close {115 101 116 32 120 32 34 36 123 97 123 125 34}
    array_open {115 101 116 32 120 32 36 97 40}
    array_closed {115 101 116 32 120 32 36 97 40 102 111 111 41}
    array_open_command {115 101 116 32 120 32 36 97 40 102 111 111 91 115 101 116 32 121}
    array_quote_literal {115 101 116 32 120 32 36 97 40 34 102 111 111 41}
    array_tail_open_command {115 101 116 32 120 32 36 97 40 102 111 111 41 116 97 105 108 91 115 101 116 32 121}
    expansion_open_brace {115 101 116 32 120 32 123 42 125 123}
    expansion_open_quote {115 101 116 32 120 32 123 42 125 34}
    expansion_closed {115 101 116 32 120 32 123 42 125 123 97 32 98 125}
    trailing_backslash {115 101 116 32 120 32 92}
    pending_continuation {115 101 116 32 120 32 92 10}
    comment_pending_continuation {35 32 99 92 10}
    comment_literal_delimiters {35 32 123 32 91 32 34}
    comment_crlf_continuation {35 32 99 92 13 10}
    lone_cr {115 101 116 32 120 32 123 97 13 98 125}
    crlf_brace {115 101 116 32 120 32 123 97 13 10 98 125}
    nul_before_open {115 101 116 32 120 32 49 0 123}
    nul_inside_brace {115 101 116 32 120 32 123 97 0}
    nul_after_open_command {115 101 116 32 120 32 91 0}
    nul_literal {115 101 116 32 120 32 97 0 98}
    literal_variable_dollar {115 101 116 32 120 32 36}
    stray_close_brace {115 101 116 32 120 32 125}
    unicode_non_ascii {115 101 116 32 120 32 233}
    jim_sugar_open {115 101 116 32 120 32 36 40 49 43 50}
    jim_sugar_closed {115 101 116 32 120 32 36 40 49 43 50 41}
}
foreach {label points} $cases {
    set original ""
    foreach point $points {
        append original [format %c $point]
    }
    set status [catch {info complete $original} result]
    puts "ROW\t$label\t$status\t$result"
}
