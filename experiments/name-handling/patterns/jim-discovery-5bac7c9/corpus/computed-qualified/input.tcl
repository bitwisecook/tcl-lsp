source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b70726f6320656d6974207b7d207b72657475726e2066697273747d7d0a736574206e73203a3a6e680a736574206865616420247b6e737d3a3a656d6974]
set question [binary format H* 2468656164]
set setup_code [catch $setup setup_result setup_options]
set question_code not-run
set question_result {}
set question_options {}
if {!$setup_code} {
    set question_code [catch $question question_result question_options]
}
foreach field {setup_code setup_result setup_options question_code question_result question_options} {
    binary scan [set $field] H* bytes
    puts [list FIELD $field $bytes]
}
puts [list JIMTEST [array get ::testinfo]]
