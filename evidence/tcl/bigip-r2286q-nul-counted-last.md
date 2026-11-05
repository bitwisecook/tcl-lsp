# BIG-IP multi-event naming: nul counted last

## Question

When a variable name containing an embedded NUL is written after its shorter prefix, do the two names refer to distinct cells through reads, upvar and unset? This affects variable-name resolution in the LSP, compiler and VM.

## Assumptions

The payload was evaluated under catch inside event-specific wrappers. Event results were reassembled only when all parts were present and unambiguous. Results from one event, unit or version do not establish another. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-8](fixtures/bigip-tmm-traffic-event-8.md)

- run: `r2286q_nul_`

## Results

### TMM 0:0, CLIENT_ACCEPTED; TMM 0:1, CLIENT_ACCEPTED; TMM 0:2, CLIENT_ACCEPTED; TMM 0:3, CLIENT_ACCEPTED

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule CLIENT_ACCEPTED event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_counted 0 {}} {after_unset 27 0 PLAIN} {after_unset 29 1 {can't read "__tcl_lsp_2286_r2286q_nul_A⟦U+0000⟧B": no such variable}}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 2 log parts; the retained log contained these numbers of copies per part: [32, 32].

### TMM 0:0, RULE_INIT; TMM 0:1, RULE_INIT; TMM 0:2, RULE_INIT; TMM 0:3, RULE_INIT

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule RULE_INIT event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_counted 0 {}} {after_unset 27 0 PLAIN} {after_unset 29 1 {can't read "__tcl_lsp_2286_r2286q_nul_A⟦U+0000⟧B": no such variable}}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 2 log parts; the retained log contained these numbers of copies per part: [1, 1].

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST

- Recorded version: Not separately recorded in this HTTP capture.
- Recorded input channel: iRule HTTP_REQUEST event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_counted 0 {}} {after_unset 27 0 PLAIN} {after_unset 29 1 {can't read "__tcl_lsp_2286_r2286q_nul_A⟦U+0000⟧B": no such variable}}⟦EOF⟧
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
{names 27 29} {write 27 0 PLAIN} {write 29 0 COUNTED} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_counted 0 {}} {after_unset 27 0 PLAIN} {after_unset 29 1 {can't read "__tcl_lsp_2286_r2286q_nul_A⟦U+0000⟧B": no such variable}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0; the same result was recorded in 32 HTTP responses with status HTTP/1.0 200 OK.
