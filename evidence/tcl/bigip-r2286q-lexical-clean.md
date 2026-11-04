# BIG-IP multi-event naming: lexical clean

## Question

How do the tested format and binary byte producers affect braced and unbraced variable-token parsing when the source is constructed dynamically? This affects lexical name recognition in the LSP and compiler.

## Assumptions

The payload was evaluated under catch inside event-specific wrappers. Event results were reassembled only when all parts were present and unambiguous. Results from one event, unit or version do not establish another. Other assumptions were not separately recorded.

## Script

```tcl-visible
set prefix __tcl_lsp_2286_r2286q_lexclean_
set rows {}
foreach pair [list [list format_pre [format %c 233]] [list format_comb [format %c 769]] [list bytes_pre [binary format H* c3a9]] [list bytes_comb [binary format H* cc81]] [list nul [binary format H* 0042]]] {
    set label [lindex $pair 0]
    set suffix [lindex $pair 1]
    set name $prefix
    append name $suffix
    set seed_rc [catch {set $prefix SHORT_PREFIX} seed_result]
    set write_rc [catch {set $name FULL_VALUE} write_result]
    foreach form {unbraced braced} {
        set script "set observed \$"
        if {$form eq "braced"} {
            append script "\{" $name "\}"
        } else {
            append script $name
        }
        binary scan $script H* script_hex
        set rc [catch {eval $script} result]
        binary scan $result H* result_hex
        lappend rows [list $label $form seed $seed_rc write $write_rc script_hex $script_hex eval $rc result_hex $result_hex]
    }
    catch {unset $name}
    catch {unset $prefix}
    catch {unset observed}
}
set rows
```

## Results

### TMM 0:0, CLIENT_ACCEPTED

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule CLIENT_ACCEPTED event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 10 log parts; the retained log contained these numbers of copies per part: [32, 32, 32, 32, 32, 32, 32, 32, 32, 32].

### TMM 0:1, CLIENT_ACCEPTED

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule CLIENT_ACCEPTED event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 10 log parts; the retained log contained these numbers of copies per part: [32, 32, 32, 32, 32, 32, 32, 32, 32, 32].

### TMM 0:2, CLIENT_ACCEPTED

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule CLIENT_ACCEPTED event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 10 log parts; the retained log contained these numbers of copies per part: [32, 32, 32, 32, 32, 32, 32, 32, 32, 32].

### TMM 0:3, CLIENT_ACCEPTED

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule CLIENT_ACCEPTED event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 10 log parts; the retained log contained these numbers of copies per part: [32, 32, 32, 32, 32, 32, 32, 32, 32, 32].

### TMM 0:0, RULE_INIT

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule RULE_INIT event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 10 log parts; the retained log contained these numbers of copies per part: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1].

### TMM 0:1, RULE_INIT

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule RULE_INIT event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 10 log parts; the retained log contained these numbers of copies per part: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1].

### TMM 0:2, RULE_INIT

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule RULE_INIT event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 10 log parts; the retained log contained these numbers of copies per part: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1].

### TMM 0:3, RULE_INIT

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule RULE_INIT event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 10 log parts; the retained log contained these numbers of copies per part: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1].

### TMM 0:0, HTTP_REQUEST

- Recorded version: Not separately recorded in this HTTP capture.
- Recorded input channel: iRule HTTP_REQUEST event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0; the same result was recorded in 32 HTTP responses with status HTTP/1.0 200 OK.

### TMM 0:1, HTTP_REQUEST

- Recorded version: Not separately recorded in this HTTP capture.
- Recorded input channel: iRule HTTP_REQUEST event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0; the same result was recorded in 32 HTTP responses with status HTTP/1.0 200 OK.

### TMM 0:2, HTTP_REQUEST

- Recorded version: Not separately recorded in this HTTP capture.
- Recorded input channel: iRule HTTP_REQUEST event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0; the same result was recorded in 32 HTTP responses with status HTTP/1.0 200 OK.

### TMM 0:3, HTTP_REQUEST

- Recorded version: Not separately recorded in this HTTP capture.
- Recorded input channel: iRule HTTP_REQUEST event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{format_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe9 eval 0 result_hex 53484f52545f505245464958e9} {format_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fe97d eval 0 result_hex 46554c4c5f56414c5545} {format_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f01 eval 0 result_hex 53484f52545f50524546495801} {format_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f017d eval 0 result_hex 46554c4c5f56414c5545} {bytes_pre unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a9 eval 0 result_hex 53484f52545f505245464958c3a9} {bytes_pre braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fc3a97d eval 0 result_hex 46554c4c5f56414c5545} {bytes_comb unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc81 eval 0 result_hex 53484f52545f505245464958cc81} {bytes_comb braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5fcc817d eval 0 result_hex 46554c4c5f56414c5545} {nul unbraced seed 0 write 0 script_hex 736574206f6273657276656420245f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f0042 eval 0 result_hex 53484f52545f5052454649580042} {nul braced seed 0 write 0 script_hex 736574206f6273657276656420247b5f5f74636c5f6c73705f323238365f7232323836715f6c6578636c65616e5f00427d eval 0 result_hex 46554c4c5f56414c5545}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0; the same result was recorded in 32 HTTP responses with status HTTP/1.0 200 OK.
