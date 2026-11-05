# BIG-IP follow-up: nul array index

## Question

Does an embedded NUL distinguish an array index from its shorter prefix through writes, reads, links and unsets? This affects array-element resolution in the LSP, compiler and VM.

## Assumptions

The retained payload ran under catch in the BIG-IP wrappers for the contexts below. Its result is limited to the recorded inputs and contexts; it does not establish behavior in another environment. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-6](fixtures/bigip-tmm-traffic-event-6.md)

- run: `r2286m_nul_array_index_trace`
- run.2: `r2286m_arrayindex_`

### callback_body

```tcl-visible
binary scan $name1 H* root_hex
binary scan $name2 H* index_hex
log local0. "R2286TRACE|r2286m|nul_array_index|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|root_hex=$root_hex|index_hex=$index_hex|op=$op"⟦EOF⟧
```

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286m_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.

### TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{indexes 1 3} {trace_proc 0 {}} {trace_add 0 {}} {write 40 0 PLAIN} {write 42 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_index 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286m_arrayindex_root(A⟦U+0000⟧B)": no such element in array}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.
