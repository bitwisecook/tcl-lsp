source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a20202020666f7265616368206e616d65207b72656420626c75657d207b70726f6320246e616d65207b7d205b6c6973742072657475726e20246e616d655d7d0a7d]
set question [binary format H* 6c697374205b3a3a6e683a3a7265645d205b3a3a6e683a3a626c75655d]
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
