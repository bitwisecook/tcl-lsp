source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a202020202020202070726f632061737369676e207b6e616d657d207b757076617220246e616d652076616c75653b207365742076616c7565206368616e6765647d0a202020202020202070726f632063616c6c6572207b7d207b736574203061206f6c643b206361746368207b61737369676e2030617d7d0a202020202020202063616c6c65720a20202020]
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
