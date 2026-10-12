source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a20202020202020206e616d657370616365206576616c206e207b0a20202020202020202020202070726f63206c6973742061726773207b72657475726e207b6f746865727d7d0a20202020202020202020202070726f632074616b652061726773207b72657475726e20696e74656e6465647d0a20202020202020202020202070726f63206f746865722061726773207b72657475726e2061637475616c7d0a2020202020202020202020206576616c205b6c6973742074616b65207b6120627d5d0a20202020202020207d0a20202020]
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
