# Connector-content byte provenance correction

The historical decoded log captures each have exactly one appended LF introduced during root capture transfer. This directory preserves the canonical exact UTF8 connector content in gzip files and maps every old/new raw and compressed hash in `mapping-addendum.json`.

All13 comparisons prove there is no other byte difference. Nonempty lines and parsed terminal outcome records, including repeated reporter displays, are identical; all classifications, exact selector joins and counts remain unchanged.

The initial eight were independently fetched again through the read-only GitHub connector and written through explicit UTF8/base64 conversion. The later five use root-retained original connector content reconstructed with the same byte-preserving method. Every gzip was decompressed and compared byte-for-byte.

Historical archives and receipts remain unchanged. Their hashes identify the LF-appended captures; this addendum supersedes only prior claims that those hashes represented the exact original connector-content bytes. It changes no test or release verdict.
