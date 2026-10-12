source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a202020202020202070726f6320756e6b6e6f776e207b6865616420617267737d207b72657475726e205b6c696e7365727420246172677320302024686561645d7d0a20202020202020206c697374205b696e666f20636f6d6d616e647320616273656e745d205b616273656e742066697865645d0a20202020]
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
