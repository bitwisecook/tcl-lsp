# Exact source patch chunks

The numbered files are consecutive byte-exact pieces of the original
`source.patch`. Each piece is at most 4 MiB. The manifest records their order,
lengths and SHA-256 hashes, and the complete patch hash matches
`../receipt.json` without changing its source-capture claim.

From this directory reconstruct outside the repository:

```sh
cat [0-9][0-9][0-9][0-9] > /tmp/name-native-ensemble-source.patch
sha256sum /tmp/name-native-ensemble-source.patch
```

Check the complete hash against `manifest.json` and `../receipt.json` before
using the reconstructed patch. Chunk boundaries may fall within a line.
