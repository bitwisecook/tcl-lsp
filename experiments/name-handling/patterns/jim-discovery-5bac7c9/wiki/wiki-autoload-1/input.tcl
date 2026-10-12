source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a2020202020202020736574206175746f5f696e646578283a3a6c61746529207b70726f63203a3a6c617465207b7d207b72657475726e206c6f616465647d7d0a20202020202020206c697374205b696e666f20636f6d6d616e6473203a3a6c6174655d205b3a3a6c6174655d205b696e666f20636f6d6d616e6473203a3a6c6174655d0a20202020]
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
