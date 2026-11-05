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
   executed script in the notation below, or a link to a shared fixture plus
   the values this case supplies (see [Fixtures](#fixtures)).
4. **Results** — one level-three subsection per recorded environment, naming
   the environment and its recorded version or `Not recorded`. Include the
   observed exit status, standard output and standard error. Distinguish empty
   output from output that was not recorded. Record the invocation or channel
   when known; do not invent one when it is absent. BIG-IP is included only
   when an actual BIG-IP execution was recorded. If the environment reports
   results through another channel, add a **Recorded result** `text-visible`
   block and explain precisely what it contains. A browser projection, for
   example, is not standard output; mark unrecorded process streams and exit
   status as `Not recorded`. When a stream or recorded result consists only of
   encoded rows whose every field is decoded in the entries that follow, write
   `decoded below` instead of repeating the raw rows.

Observations establish only what that run measured. They do not by themselves
establish that the LSP, compiler, VM, another Tcl version, or another execution
channel behaves the same way. Describe that boundary in the question or
assumptions when relevant.

Keep results minimal. Record the output the question depends on, not whole
configurations, complete logs or build receipts. Do not record content hashes:
the bytes in git are the record. A hash belongs in evidence only when the hash
itself is what the experiment measured.

## Fixtures

When many cases run inside the same harness, put the harness in one file under
[`evidence/tcl/fixtures`](../../../evidence/tcl/fixtures/README.md) and keep each
case file to what that case changes. A fixture has:

- **Setup** — how to deploy and trigger the harness, and how its results are
  collected.
- **Template** — one `tcl-visible` block. `⟦HEX:name⟧` is replaced by the
  lowercase hexadecimal form of the named value; `⟦PARAM:name⟧` by its exact
  bytes.
- **Constants** (optional) — `### name` headings, each with one `tcl-visible`
  block, for values every case shares.

The case's `## Script` section starts with `Fixture: name`
and supplies every other slot, either as `- name: \`value\`` for a short plain
value or as a `### name` heading with one `tcl-visible` block. Every slot must
be filled exactly once and every value used. The composed bytes are the
executed script; the checker and extractor compose them. Do not create a
fixture that only one case uses.

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

## Older records

The [name-resolution proof pages](../analysis/name-resolution-proofs/README.md)
predate this format. They keep each question's conclusion, provider answers and
code bindings, and point at native captures retained as test data. The raw
appliance and run archives they once cited have been removed; their BIG-IP
executions were converted into files under `evidence/tcl`. Record new
executions here, not as proof pages.
