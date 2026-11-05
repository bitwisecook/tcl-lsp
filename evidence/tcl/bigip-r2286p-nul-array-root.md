# BIG-IP cross-context naming: nul array root

## Question

Does an embedded NUL distinguish an array root from its shorter prefix through writes, reads, links and unsets? This affects array-root resolution in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-7](fixtures/bigip-tmm-traffic-event-7.md)

- run: `r2286p_nul_array_root_trace`
- run.2: `r2286p_arrayroot_`

### callback_body

```tcl-visible
binary scan $name1 H* root_hex
binary scan $name2 H* index_hex
log local0. "R2286TRACE|r2286p|nul_array_root|tmm=[TMM::cmp_group]:[TMM::cmp_unit]|root_hex=$root_hex|index_hex=$index_hex|op=$op"⟦EOF⟧
```

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{roots 33 35} {trace_proc 0 {}} {trace_add 0 {}} {write 36 0 PLAIN} {write 38 0 COUNTED} {exists plain 1} {read plain 0 PLAIN} {exists counted 1} {read counted 0 COUNTED} {upvar_write 0 COUNTED_MUTATION} {after_link plain 0 PLAIN} {after_link counted 0 COUNTED_MUTATION} {unset_counted_root 0 {}} {after_unset_exists plain 1} {after_unset_read plain 0 PLAIN} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A⟦U+0000⟧B(k)": no such variable}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result was recorded in 32 responses with status HTTP/1.0 200 OK.

### IAppImplementation; ICallScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: IAppImplementation; ICallScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{roots 33 35} {trace_proc 0 {}} {trace_add 0 {}} {write 36 1 {can't set "__tcl_lsp_2286_r2286p_arrayroot_A(k)": invalid command name "TMM::cmp_group"}} {write 38 1 {can't set "__tcl_lsp_2286_r2286p_arrayroot_A⟦U+0000⟧B(k)": invalid command name "TMM::cmp_group"}} {exists plain 1} {read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A(k)": invalid command name "TMM::cmp_group"}} {exists counted 1} {read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A⟦U+0000⟧B(k)": invalid command name "TMM::cmp_group"}} {upvar_write 1 {can't set "alias(k)": invalid command name "TMM::cmp_group"}} {after_link plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A(k)": invalid command name "TMM::cmp_group"}} {after_link counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A⟦U+0000⟧B(k)": invalid command name "TMM::cmp_group"}} {unset_counted_root 0 {}} {after_unset_exists plain 1} {after_unset_read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A(k)": invalid command name "TMM::cmp_group"}} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A⟦U+0000⟧B(k)": no such variable}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.

### TmshCliScript

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TmshCliScript.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{roots 33 35} {trace_proc 0 {}} {trace_add 0 {}} {write 36 1 {can't set "__tcl_lsp_2286_r2286p_arrayroot_A(k)": invalid command name "TMM::cmp_group"}} {write 38 1 {can't set "__tcl_lsp_2286_r2286p_arrayroot_A⟦U+0000⟧B(k)": invalid command name "TMM::cmp_group"}} {exists plain 1} {read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A(k)": invalid command name "TMM::cmp_group"}} {exists counted 1} {read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A⟦U+0000⟧B(k)": invalid command name "TMM::cmp_group"}} {upvar_write 1 {can't set "alias(k)": invalid command name "TMM::cmp_group"}} {after_link plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A(k)": invalid command name "TMM::cmp_group"}} {after_link counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A⟦U+0000⟧B(k)": invalid command name "TMM::cmp_group"}} {unset_counted_root 0 {}} {after_unset_exists plain 1} {after_unset_read plain 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A(k)": invalid command name "TMM::cmp_group"}} {after_unset_exists counted 0} {after_unset_read counted 1 {can't read "__tcl_lsp_2286_r2286p_arrayroot_A⟦U+0000⟧B(k)": no such variable}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
