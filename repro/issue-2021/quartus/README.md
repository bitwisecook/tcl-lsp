# Quartus IP-library corpus results (issue #2021)

Corpus: the Tcl-family files (`.tcl .qsf .qpf .qip .sdc`) from `ip/altera` of
the official `alterafpga/quartus-std:25.1std-max10` image, streamed from the
Docker Hub registry with `fetch.sh` (not committed: 3214 files, 97 MiB,
Intel-licensed). The reporter's 23.1std tree has 3217 files.

- `scan-eof-midscan.samples.txt` — the harness run: client torn down 1.5 s
  after `initialized`; the server stayed at ~400 % CPU for the 32 min it was
  allowed to run (7640 CPU-s), RSS flat at 3.2 GiB from minute 12.
- `scan-sample{1,2,3-deep}.gdb.txt` — thread stacks at 0.5, 30 and 31 min.
  At 30 min all four busy threads are ~100 frames deep in
  `tcl_compiler::analyser::diagnostics::helpers::phi_can_undef`.
- `perfile-timing.tsv` — `tcl diag` per file (wall s, peak RSS KiB, lines,
  status, path; 90 s cap): 7 files never finish, 7 more take 40–90 s, the
  other 3200 total 3374 s. Produced by `../time_files.py`.
