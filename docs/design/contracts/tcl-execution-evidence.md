# Tcl execution evidence

Evidence is a script that was actually run, the Tcl environments in which it
was run, and the results observed in each environment. Keep one standalone
Markdown file per collected piece of evidence. Group the results from the
environments that ran that script in that file. Do not infer an unrun
environment, missing output, an exit status, an invocation channel, or an
assumption from another run.

Implementation code, source reviews, generated reports, inventories, build
metadata and proof annotations are development material. They may help develop
or explain the project, but they are not Tcl execution evidence. Existing
material may be retained without being presented as a captured run.

## Required sections

Each evidence file has these level-two sections:

1. **Question** — the full semantic question the script was designed to answer,
   including why the distinction matters to the LSP, compiler or VM. State
   the scope narrowly enough that the observations cannot be mistaken for an
   answer to a broader question.
2. **Assumptions** — every assumption made in the experiment and any relevant
   limitation. Write `Not recorded` where the retained capture does not say;
   do not turn an unknown assumption into `None`.
3. **Script** — exactly one `tcl-visible` fenced block containing the exact
   executed script in the notation below.
4. **Results** — one level-three subsection per recorded environment, naming
   the environment and its recorded version or `Not recorded`. Include the
   observed exit status, standard output and standard error. Distinguish empty
   output from output that was not recorded. Record the invocation or channel
   when known; do not invent one when it is absent. BIG-IP is included only
   when an actual BIG-IP execution was recorded. If the environment reports
   results through another channel, add a **Recorded result** `text-visible`
   block and explain precisely what it contains. A browser projection, for
   example, is not standard output; mark unrecorded process streams and exit
   status as `Not recorded`.

Observations establish only what that run measured. They do not by themselves
establish that the LSP, compiler, VM, another Tcl version, or another execution
channel behaves the same way. Describe that boundary in the question or
assumptions when relevant.

## Visible byte notation

Evidence files themselves are UTF-8 Markdown with LF line endings. A physical
line break in a visible block normally represents an LF byte in the captured
script or stream. Normal readable Unicode, including Cyrillic, Greek, Arabic,
CJK and other human-language text, is written literally and is never silently
normalized. Use character notes when a question depends on a confusable or
combining character; for example, distinguish Latin `a` from Cyrillic `а` by
name and code point.

The following markers are reserved inside `tcl-visible` and `text-visible`
blocks:

| Marker | Original bytes or character |
| --- | --- |
| `⟦NBSP⟧` | U+00A0 no-break space, encoded as UTF-8 |
| `⟦SPACE⟧` | One trailing ASCII space |
| `⟦TAB⟧` | Tab byte |
| `⟦CRLF⟧` | CRLF line ending; the following Markdown line break is only layout |
| `⟦CR⟧` | Bare CR byte |
| `⟦EOF⟧` | End of bytes without a final LF; the following Markdown line break is only layout |
| `⟦U+XXXX⟧` | The named Unicode scalar encoded as UTF-8, for other invisible or control characters |
| `⟦BYTE:HH⟧` | One byte that cannot be decoded as UTF-8, in uppercase hexadecimal |
| `⟦LBRACKET⟧`, `⟦RBRACKET⟧` | A literal `⟦` or `⟧` in the original bytes |

Use `⟦SPACE⟧` for every trailing space before an LF, CRLF, bare CR or end of
file. Interior ordinary spaces remain literal. Tabs are always marked. A
literal marker is therefore never confused with notation. Empty bytes are
represented by `⟦EOF⟧`. Fences must be longer than any consecutive run of
backticks in the represented data. `⟦CRLF⟧` and `⟦EOF⟧` occur only at the end
of a displayed line; `⟦EOF⟧` occurs only on the last displayed line.

For example, the first line below contains an NBSP, a trailing space and a
CRLF. The second has an ordinary LF:

```text
puts "Привет,⟦NBSP⟧мир"⟦SPACE⟧⟦CRLF⟧
puts "Готово"
```

## Tools

Create a script block from the original bytes:

```sh
python3 scripts/dev/tcl-script-to-block.py path/to/script.tcl -o /tmp/script-block.md
```

Copy that block into the evidence file's `## Script` section. Use `--kind
output` to encode a captured output stream as a `text-visible` block. Question,
assumptions, environment and results must be supplied from the actual
experiment; the tool does not infer them.

Recover the original script bytes:

```sh
python3 scripts/dev/extract-tcl-evidence.py evidence/tcl/example.md -o /tmp/recovered.tcl
```

Use `-o -` only when a binary standard-output stream is appropriate. The
extractor rejects malformed or ambiguous markers rather than guessing. These
tools do not need a content hash to reconstruct the bytes.

`make check-tcl-evidence` checks the required sections, per-environment result
fields and reversible visible blocks in the standalone files under
`evidence/tcl`. It cannot decide whether a question is sufficiently meaningful
or whether a claimed execution actually occurred; authors and reviewers must
check those facts against the retained run. It checks the files that exist, not
whether every historical capture has already been converted.

## Existing captures

The older name-resolution records mix captured runs with development notes and
other material. They are not automatically converted into execution evidence:
some do not identify a Tcl script or a measured result unambiguously. Migrate a
capture only when its full question, actual script, environments and recorded
results can be recovered. Mark missing assumptions or details as not recorded.
Keep the source capture until that conversion has been checked; do not fabricate
a result merely to complete a migration.

The current migration is incomplete. In particular, concatenated BIG-IP
callback sources need to be split into the individual callbacks that ran,
browser projections must be identified as such, and some records refer to
several scripts or to a failed setup rather than a reached operation. The
conservative importer can list records it cannot convert:

```sh
python3 scripts/dev/migrate-tcl-evidence.py --report /tmp/tcl-evidence-review.json
```

That report tracks records handled by this importer, not coverage of every
capture in the repository. Some listed records may have been converted by a
separate, more specific migration; others may be development material rather
than executions. Review the original material before treating a listed record
as an outstanding run.
