# Runtime cleanup measurement: nh20261012repair1

[cases.json](cases.json) identifies the exact check and cleanup of cap1's
run-owned namespace and global command names. The generated input,
configuration, driver and raw log files are under `tmm-RULE_INIT/`.

Units `0:0`, `0:1`, `0:2` and `0:3` each reported lookup completion 0, with
the namespace present and the two global names
`::__nh_nh20261012cap1_target` and `::__nh_nh20261012cap1_trace` present.
Each unit then reported runtime-cleanup completion 0 and result hex
`30207b7d` (`0 {}` in ASCII). These fields use the recorded
`binary-scan-tcl-value` encoding. The configuration rule's post-delete check
reports that its exact name was not found.

This is a bounded cleanup observation for those names on those four units.
It supplies no missing cap1 lookup, APL execution, provider implementation
explanation or assertion about unrelated interpreter state. No appliance
version/build receipt is present in this repair run; it must not borrow one
as its own measurement. `MANIFEST.sha256` seals the retained files.
