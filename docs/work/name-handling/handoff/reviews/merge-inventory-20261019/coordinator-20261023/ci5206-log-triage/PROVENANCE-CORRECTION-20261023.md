# Connector log byte provenance correction

The [canonical correction addendum](connector-byte-correction-20261023/mapping-addendum.json) supersedes prior exact-original-content claims for13 CI5206 decoded logs. Each historical capture contains exactly one appended LF; canonical gzip archives now preserve the original connector UTF8 bytes.

The historical archives, hashes and classifications remain intact. Independent comparisons prove all nonempty lines and terminal outcomes unchanged, so counts and exact selector joins do not change. See [correction details](connector-byte-correction-20261023/README.md).
