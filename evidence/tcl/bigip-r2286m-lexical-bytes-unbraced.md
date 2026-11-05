# BIG-IP follow-up: lexical bytes unbraced

## Question

How does Tcl parse an unbraced variable token containing name bytes produced by binary format? This affects lexical name recognition in the LSP and compiler.

## Assumptions

The retained payload ran under catch in the BIG-IP wrappers for the contexts below. Its result is limited to the recorded inputs and contexts; it does not establish behavior in another environment. Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-14](fixtures/bigip-tmm-traffic-event-14.md)

- run: `r2286m_lex_bytes_`

## Results

### TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST; TMM 0:1, HTTP_REQUEST; TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
{pre write 0 PRE_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735fc3a9 eval 1 63616e2774207265616420225f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f223a206e6f2073756368207661726961626c65} {decomposed write 0 DECOMPOSED_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f65cc81 eval 1 63616e2774207265616420225f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f65223a206e6f2073756368207661726961626c65}⟦EOF⟧
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
{pre write 0 PRE_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735fc3a9 eval 1 63616e2774207265616420225f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f223a206e6f2073756368207661726961626c65} {decomposed write 0 DECOMPOSED_VALUE script_hex 73657420245f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f65cc81 eval 1 63616e2774207265616420225f5f74636c5f6c73705f323238365f72323238366d5f6c65785f62797465735f65223a206e6f2073756368207661726961626c65}⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The same result appeared in 32 responses with status HTTP/1.0 200 OK.
