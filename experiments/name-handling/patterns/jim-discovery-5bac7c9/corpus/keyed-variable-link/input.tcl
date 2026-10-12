source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a20202020736574207b73746174652e6120622876616c7565297d206f726967696e616c0a2020202070726f63206368616e6765207b6e616d657d207b0a20202020202020207570766172203a3a6e683a3a246e616d652073746174650a20202020202020207365742073746174652876616c756529206368616e6765640a202020207d0a7d0a3a3a6e683a3a6368616e6765207b73746174652e6120627d]
set question [binary format H* 736574207b3a3a6e683a3a73746174652e6120622876616c7565297d]
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
