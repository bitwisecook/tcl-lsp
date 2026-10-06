# Measured rule-source images

These complete rule-body words preserve the exact bytes between the configuration
braces in the Unicode controls recorded in
[BIGIP_RESULTS.md](../../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md).
The opening and trailing LF are retained. The corresponding configuration files,
hashes, load transcripts and four-TMM traffic are in
[the raw appliance evidence](../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/).

The Registry's explicitly selected BIG-IP 21.1.0.1 build 0.0.26 loader profile
recognizes only these complete images. The three grinning/skin-tone/flag emoji
images have measured ACCEPT outcomes; the other ten have measured REJECT outcomes.
Changed images and other non-ASCII source remain unsupported. This corpus proves
no general Unicode identifier grammar, normalization, key identity, native parser
recipe, object representation or compiler registration.
