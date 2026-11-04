# Tcl naming pattern: oo-prefix-deleted-near-miss

## Question

What happens to a command prefix containing an object's self command after that object is destroyed: can it still invoke the method or does command lookup fail?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. The Jim wrapper also sources the test helper named in the script; the result depends on that helper. Other assumptions were not separately recorded.

## Script

```tcl-visible
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6f6f3a3a636c61737320637265617465203a3a6e68636c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b73656c665d20656d6974207b6120627d7d0a7d0a736574206f626a656374205b3a3a6e68636c617373206e65775d0a73657420707265666978205b246f626a656374207072656669785d0a246f626a6563742064657374726f790a6361746368207b6576616c20247072656669787d20726573756c74]
set question [binary format H* 737472696e67206d61746368207b696e76616c696420636f6d6d616e64206e616d65202a7d2024726573756c74]
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
```

## Results

### Jim 0.84-9-g5bac7c9

- Recorded version: Jim 0.84-9-g5bac7c9
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
FIELD setup_code 31
FIELD setup_result 696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
FIELD setup_options 2d636f64652031202d6c6576656c2030202d6572726f72696e666f207b7b7d207b7d2031207b7d7d202d6572726f72636f6465204e4f4e45
FIELD question_code 6e6f742d72756e
FIELD question_result {}
FIELD question_options {}
JIMTEST {verbose 0 numpass 0 stoponerror 0 template 0 numfail 0 numskip 0 numtests 0 reported 0 failed {} source input.tcl}
```

- Standard error: empty.

The intended final operation was not reached.

The following entries decode the FIELD payloads in standard output.

- setup_code:

```text-visible
1⟦EOF⟧
```

- setup_result:

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

- setup_options:

```text-visible
-code 1 -level 0 -errorinfo {{} {} 1 {}} -errorcode NONE⟦EOF⟧
```

- question_code:

```text-visible
not-run⟦EOF⟧
```

- question_result:

```text-visible
⟦EOF⟧
```

- question_options:

```text-visible
⟦EOF⟧
```
