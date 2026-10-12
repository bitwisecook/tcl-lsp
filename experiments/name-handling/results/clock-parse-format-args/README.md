# Private ParseFormatArgs: eleven exact inputs

Question: after matching real startup, what does the named private
`::tcl::clock::ParseFormatArgs` worker answer for [this unchanged eleven-case
input](input.tcl), and which selected releases lack that worker?

[source-only-review.md](source-only-review.md) freezes the preceding source
review as the question's basis. It is explicitly not a runtime result.
[records.json](captured/records.json) retains actual argv, source/input/binary/
library hashes, exit codes and stream hashes; [status.json](status.json) records
validated counts and limits. Six fresh stock processes executed once each,
with real matching Tcl_Init/library trees or pinned Jim's actual shell startup.
All six processes exited0, emitted11 caught-case rows and empty stderr; this
is process completion, not eleven semantic successes.

| Provider | Named private worker observation |
| --- | --- |
| C8.4.20 | All11 command-absence errors; errorCode NONE |
| C8.5.19 | Four successes/seven worker errors; bad/ambiguous **switch**, CLOCK badSwitch |
| C8.6.18 | Four successes/seven worker errors; bad/ambiguous **option**, CLOCK badOption |
| C9.0.4 / C9.1.0 | All11 command-absence errors; TCL LOOKUP COMMAND |
| Jim0.84, pinned5bac7c99ad65864c87da513e22e2f01703fa4e03 | All11 command-absence errors; errorCode NONE |

C8.5/C8.6 both return the default three-element list, accept `-g`, let the
last repeated format win, and reject even false `-gmt` together with timezone.
Bad GMT conversion precedes bad clockval; bad clockval precedes the mutual
exclusion error. Missing arguments/value use CLOCK wrongNumArgs.
**Both accept clockval9223372036854775808**, returning the default list;
a proposed implementation must not fabricate a signed-range rejection.
Success leaves the explicitly seeded errorCode UNSET. Exact messages and
object string values remain in the raw stdout files, without normalisation.

Absence of this private worker does not answer public `clock format` on C8.4,
C9 or Jim. These post-startup script observations do not establish constructor
hooks, physical registration order, pool identity or object lifetime; the prior
C8.6 C API constructor capture and source review answer different questions.
Source excerpts preserve raw tabs/bytes with complete source hashes and line
ranges in [source-excerpts.json](captured/source-excerpts.json). Binary and
library hashes matched before/after; selected configured source hashes do not
independently authenticate embedded binary bytes. No F5 result is measured or
inferred. No runtime repair, Rust/build, tcltest suite or native runner occurred.

Replay into a REQUIRED fresh destination; no retained result is overwritten:

```sh
python experiments/name-handling/results/clock-parse-format-args/capture.py \
  --output-dir /tmp/clock-parse-format-args-replay-NEW
```
