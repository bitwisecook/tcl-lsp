source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a202020202020202070726f63206275696c64207b7d207b0a202020202020202020202020666f7265616368206e616d65207b616c70686120626574617d207b70726f63203a3a246e616d65207b7d207b72657475726e20246e616d657d7d0a20202020202020207d0a20202020202020206275696c640a20202020202020206361746368207b616c7068617d0a20202020]
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
