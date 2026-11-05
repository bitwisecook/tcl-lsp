# BIG-IP counted names: counted input boundaries

## Question

Do event-local scalar names containing embedded NUL bytes remain distinct from their shorter prefixes across writes, reads and unsets? This determines which variable-name distinctions the LSP, compiler and VM may model in this BIG-IP context.

## Assumptions

This is the original event rule. The retained syslog lines stop before the full result list is complete; only the visible prefix of each line was recorded. The measured context is the HTTP_REQUEST event on the recorded BIG-IP appliance. Results from other contexts or versions are not implied. Other assumptions were not separately recorded.

## Script

```tcl-visible
when HTTP_REQUEST {
    set nul [format %c 0]
    set rows {}
    foreach spec {
        {short x}
        {long __resolution2286_context_invariance_long_ascii_prefix_0123456789}
        {different relocated_2286_key}
    } {
        set label [lindex $spec 0]
        set plain [lindex $spec 1]
        set name $plain
        append name $nul B $nul C
        catch {unset $plain}
        catch {unset $name}
        set plain_rc [catch {set $plain PLAIN} plain_result]
        set name_rc [catch {set $name COUNTED} name_result]
        foreach input [list $plain $name] {
            binary scan $input H* input_hex
            set rc [catch {set $input} result]
            binary scan $result H* result_hex
            lappend rows [list $label name_hex $input_hex write_plain $plain_rc write_counted $name_rc read $rc result_hex $result_hex]
        }
        catch {unset $name}
        set rc [catch {set $plain} result]
        binary scan $result H* result_hex
        lappend rows [list $label after_counted_unset $rc result_hex $result_hex]
        catch {unset $plain}
    }
    set inputs [list {} {()} {(k)} {A()} {A(k)} {A(k(l))} {A(k)tail} {:} {A:B} {A::B}]
    foreach input $inputs {
        binary scan $input H* input_hex
        set rc [catch {set $input GRAMMAR_VALUE} result]
        binary scan $result H* result_hex
        lappend rows [list grammar name_hex $input_hex write $rc result_hex $result_hex]
        set read_rc [catch {set $input} read_result]
        binary scan $read_result H* read_hex
        lappend rows [list grammar name_hex $input_hex read $read_rc result_hex $read_hex]
        catch {unset $input}
    }
    log local0.notice [list RESOLUTION2286_COUNTED_INPUT_BOUNDARIES tmm [TMM::cmp_unit] rows $rows]
    HTTP::respond 200 content OK
}
```

## Results

### TMM unit 0, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule HTTP_REQUEST event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{{short name_hex 78 write_plain 0 write_counted 0 read 0 result_hex 504c41494e} {short name_hex 7800420043 write_plain 0 write_counted 0 read 0 result_hex 434f554e544544} {short after_counted_unset 0 result_hex 504c41494e} {long name_hex 5f5f7265736f6c7574696f6e323238365f636f6e746578745f696e76617269616e63655f6c6f6e675f61736369695f7072656669785f30313233343536373839 write_plain 0 write_counted 0 read 0 result_hex 504c41494e} {long name_hex 5f5f7265736f6c7574696f6e323238365f636f6e746578745f696e76617269616e63655f6c6f6e675f61736369695f7072656669785f3031323334353637383900420043 write_plain 0 write_counted 0 read 0 result_hex 434f554e544544} {long after_counted_unset 0 result_hex 504c41494e} {different name_hex 72656c6f63617465645f323238365f6b6579 write_plain 0 write_counted 0 read 0 result_hex 504c41494e} {different name_hex 72656c6⟦EOF⟧
```

The retained log contains this same incomplete result prefix 32 times for this unit. No result is inferred for the truncated remainder.

### TMM unit 1, HTTP_REQUEST; TMM unit 2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule HTTP_REQUEST event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{{short name_hex 78 write_plain 0 write_counted 0 read 0 result_hex 504c41494e} {short name_hex 7800420043 write_plain 0 write_counted 0 read 0 result_hex 434f554e544544} {short after_counted_unset 0 result_hex 504c41494e} {long name_hex 5f5f7265736f6c7574696f6e323238365f636f6e746578745f696e76617269616e63655f6c6f6e675f61736369695f7072656669785f30313233343536373839 write_plain 0 write_counted 0 read 0 result_hex 504c41494e} {long name_hex 5f5f7265736f6c7574696f6e323238365f636f6e746578745f696e76617269616e63655f6c6f6e675f61736369695f7072656669785f3031323334353637383900420043 write_plain 0 write_counted 0 read 0 result_hex 434f554e544544} {long after_counted_unset 0 result_hex 504c41494e} {different name_hex 72656c6f63617465645f323238365f6b6579 write_plain 0 write_counted 0 read 0 result_hex 504c41494e} {different name_hex 72656c⟦EOF⟧
```

The retained log contains this same incomplete result prefix 32 times for this unit. No result is inferred for the truncated remainder.

### TMM unit 3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule HTTP_REQUEST event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{{short name_hex 78 write_plain 0 write_counted 0 read 0 result_hex 504c41494e} {short name_hex 7800420043 write_plain 0 write_counted 0 read 0 result_hex 434f554e544544} {short after_counted_unset 0 result_hex 504c41494e} {long name_hex 5f5f7265736f6c7574696f6e323238365f636f6e746578745f696e76617269616e63655f6c6f6e675f61736369695f7072656669785f30313233343536373839 write_plain 0 write_counted 0 read 0 result_hex 504c41494e} {long name_hex 5f5f7265736f6c7574696f6e323238365f636f6e746578745f696e76617269616e63655f6c6f6e675f61736369695f7072656669785f3031323334353637383900420043 write_plain 0 write_counted 0 read 0 result_hex 434f554e544544} {long after_counted_unset 0 result_hex 504c41494e} {different name_hex 72656c6f63617465645f323238365f6b6579 write_plain 0 write_counted 0 read 0 result_hex 504c41494e} {different name_hex 72656c⟦EOF⟧
```

The retained log contains this same incomplete result prefix 32 times for this unit. No result is inferred for the truncated remainder.
