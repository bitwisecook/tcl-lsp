# BIG-IP multi-event naming: nul array index

## Question

Does an embedded NUL distinguish an array index from its shorter prefix through writes, reads, links and unsets? This affects array-element resolution in the LSP, compiler and VM.

## Assumptions

The payload was evaluated under catch inside event-specific wrappers. Event results were reassembled only when all parts were present and unambiguous. Results from one event, unit or version do not establish another. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-6](fixtures/bigip-tmm-traffic-event-6.md)

- run: `r2286q_nul_array_index_trace`
- run.2: `r2286q_arrayindex_`

### callback_body

```tcl-visible
binary scan $name1 H* root_hex
binary scan $name2 H* index_hex
log local0. "R2286TRACE|r2286q|nul_array_index|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|root_hex=$root_hex|index_hex=$index_hex|op=$op"⟦EOF⟧
```

## Results

### TMM 0:0, CLIENT_ACCEPTED; TMM 0:1, CLIENT_ACCEPTED; TMM 0:2, CLIENT_ACCEPTED; TMM 0:3, CLIENT_ACCEPTED

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule CLIENT_ACCEPTED event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286q_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 3 log parts; the retained log contained these numbers of copies per part: [32, 32, 32].

### TMM 0:0, RULE_INIT; TMM 0:1, RULE_INIT; TMM 0:2, RULE_INIT; TMM 0:3, RULE_INIT

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: iRule RULE_INIT event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286q_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
```

Tcl completion code: 0. This result was reconstructed from 3 log parts; the retained log contained these numbers of copies per part: [1, 1, 1].

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST

- Recorded version: Not separately recorded in this HTTP capture.
- Recorded input channel: iRule HTTP_REQUEST event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286q_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
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
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286q_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0; the same result was recorded in 32 HTTP responses with status HTTP/1.0 200 OK.
