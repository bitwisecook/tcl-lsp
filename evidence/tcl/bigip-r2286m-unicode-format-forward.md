# BIG-IP follow-up: unicode format forward

## Question

Do precomposed and decomposed variable names produced by format %c remain distinct when written in this order? This affects Unicode name resolution in the LSP, compiler and VM.

## Assumptions

The retained payload ran under catch in the BIG-IP wrappers for the contexts below. Its result is limited to the recorded inputs and contexts; it does not establish behavior in another environment. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-22](fixtures/bigip-tmm-traffic-event-22.md)

- run: `r2286m_unicode_`

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{lengths 31 32} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655fe9 utf8_conversion 1 {invalid command name "encoding"}} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655f6501 utf8_conversion 1 {invalid command name "encoding"}} {write_pre 0 PRECOMPOSED} {write_decomposed 0 DECOMPOSED} {read 0 PRECOMPOSED} {read 0 DECOMPOSED} {unset_pre 0 {}} {remaining_decomposed 0 DECOMPOSED}⟦EOF⟧
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
{lengths 31 32} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655fe9 utf8_conversion 1 {invalid command name "encoding"}} {raw_conversion 0 5f5f74636c5f6c73705f323238365f72323238366d5f756e69636f64655f6501 utf8_conversion 1 {invalid command name "encoding"}} {write_pre 0 PRECOMPOSED} {write_decomposed 0 DECOMPOSED} {read 0 PRECOMPOSED} {read 0 DECOMPOSED} {unset_pre 0 {}} {remaining_decomposed 0 DECOMPOSED}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.
