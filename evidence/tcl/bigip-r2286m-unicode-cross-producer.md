# BIG-IP follow-up: unicode cross producer

## Question

Do variable names produced by format %c and by binary format reach the same or different cells for the tested character values? This affects the LSP, compiler and VM's treatment of name producers.

## Assumptions

The retained payload ran under catch in the BIG-IP wrappers for the contexts below. Its result is limited to the recorded inputs and contexts; it does not establish behavior in another environment. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-21](fixtures/bigip-tmm-traffic-event-21.md)

- run: `r2286m_unicodecross_`

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{representation format_pre 36 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f646563726f73735fe9} {representation bytes_pre 37 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f646563726f73735fc3a9} {representation format_decomposed 37 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f646563726f73735f6501} {representation bytes_decomposed 38 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f646563726f73735f65cc81} {write FORMAT_PRE 0 FORMAT_PRE} {write BYTES_PRE 0 BYTES_PRE} {write FORMAT_DECOMPOSED 0 FORMAT_DECOMPOSED} {write BYTES_DECOMPOSED 0 BYTES_DECOMPOSED} {read format_pre 0 FORMAT_PRE} {read bytes_pre 0 BYTES_PRE} {read format_decomposed 0 FORMAT_DECOMPOSED} {read bytes_decomposed 0 BYTES_DECOMPOSED} {mutate_bytes_pre 0 BYTES_PRE_MUTATION} {mutate_bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {after_mutation format_pre 0 FORMAT_PRE} {after_mutation bytes_pre 0 BYTES_PRE_MUTATION} {after_mutation format_decomposed 0 FORMAT_DECOMPOSED} {after_mutation bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {unset_format_pre 0 {}} {unset_format_decomposed 0 {}} {after_format_unset bytes_pre 0 BYTES_PRE_MUTATION} {after_format_unset bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION}⟦EOF⟧
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
{representation format_pre 36 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f646563726f73735fe9} {representation bytes_pre 37 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f646563726f73735fc3a9} {representation format_decomposed 37 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f646563726f73735f6501} {representation bytes_decomposed 38 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f646563726f73735f65cc81} {write FORMAT_PRE 0 FORMAT_PRE} {write BYTES_PRE 0 BYTES_PRE} {write FORMAT_DECOMPOSED 0 FORMAT_DECOMPOSED} {write BYTES_DECOMPOSED 0 BYTES_DECOMPOSED} {read format_pre 0 FORMAT_PRE} {read bytes_pre 0 BYTES_PRE} {read format_decomposed 0 FORMAT_DECOMPOSED} {read bytes_decomposed 0 BYTES_DECOMPOSED} {mutate_bytes_pre 0 BYTES_PRE_MUTATION} {mutate_bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {after_mutation format_pre 0 FORMAT_PRE} {after_mutation bytes_pre 0 BYTES_PRE_MUTATION} {after_mutation format_decomposed 0 FORMAT_DECOMPOSED} {after_mutation bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION} {unset_format_pre 0 {}} {unset_format_decomposed 0 {}} {after_format_unset bytes_pre 0 BYTES_PRE_MUTATION} {after_format_unset bytes_decomposed 0 BYTES_DECOMPOSED_MUTATION}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.
