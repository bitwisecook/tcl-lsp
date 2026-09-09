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

## After the fix (commits `lsp: exit within a bounded grace…`, `compiler: answer the
phi-from-undef trace by reachability…`, `lsp: run the exit watchdog on an OS thread…`)

| run | before | after |
|---|---|---|
| Quartus, EOF 1.5 s after `initialized` | alive 32 min+ at 400 % (killed) | exits in 3.1 s |
| Quartus, full scan | never finished | 684–736 s (`files=2000`, the scan's file cap) |
| Quartus, `shutdown`+`exit` after the scan | alive at 100 % on one core | exits in 4.1 s |
| tcllib, `shutdown`+`exit` after the scan | 19 s tail at 100 % | exits in 1.1 s |
| `tcl diag nf_hssi_rx_pld_pcs_interface_rbc.tcl` | > 600 s | 31 s |
| `tcl diag` 105-stage reproducer | > 120 s | 1.2 s |
