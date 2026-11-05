# BIG-IP cross-context naming: nul plain last

## Question

When a shorter variable name is written after the same prefix followed by an embedded NUL, do the two names refer to distinct cells through reads, upvar and unset? This affects variable-name resolution in the LSP, compiler and VM.

## Assumptions

The retained payload was evaluated under catch inside context-specific wrappers. Result bytes below are the wrapper's recorded Tcl result, rather than standalone process stdout. A result in one context does not establish another context or version. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-17](fixtures/bigip-tmm-traffic-event-17.md)

- run: `r2286p_nul_`

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST; TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{names 27 29} {write 29 0 COUNTED} {write 27 0 PLAIN} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_counted 0 {}} {after_unset 27 0 PLAIN} {after_unset 29 1 {can't read "__tcl_lsp_2286_r2286p_nul_A⟦U+0000⟧B": no such variable}}⟦EOF⟧
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
{names 27 29} {write 29 0 COUNTED} {write 27 0 PLAIN} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_counted 0 {}} {after_unset 27 0 PLAIN} {after_unset 29 1 {can't read "__tcl_lsp_2286_r2286p_nul_A⟦U+0000⟧B": no such variable}}⟦EOF⟧
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
{names 27 29} {write 29 0 COUNTED} {write 27 0 PLAIN} {read 27 0 PLAIN} {read 29 0 COUNTED} {upvar_write 0 LINK_WRITE} {after_link 27 0 PLAIN} {after_link 29 0 LINK_WRITE} {unset_counted 0 {}} {after_unset 27 0 PLAIN} {after_unset 29 1 {can't read "__tcl_lsp_2286_r2286p_nul_A⟦U+0000⟧B": no such variable}}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. This result was decoded from the retained context marker.
