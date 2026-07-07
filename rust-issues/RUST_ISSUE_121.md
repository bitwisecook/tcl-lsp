# RUST_ISSUE_121: front-matter rendering strips raw HTML events but not `javascript:`/`data:` link destinations, so authored Markdown can still inject a click-activated script link into the report (inserted via `{{ front_matter_html | safe }}`, render.rs:192/template line 192)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/bigip-report-gen/rust/src/markdown.rs:44-45` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/bigip-report-gen/rust/src/markdown.rs:44-45 — front-matter rendering strips raw HTML events but not `javascript:`/`data:` link destinations, so authored Markdown can still inject a click-activated script link into the report (inserted via `{{ front_matter_html | safe }}`, render.rs:192/template line 192).
`[click me](javascript:alert(document.cookie))` in `--front-matter` renders as `<a href="javascript:...">` in the otherwise no-external-content report — defeating the module's stated goal ("dropping raw HTML keeps an author … from injecting script into it"). Quote: `.filter(|ev| !matches!(ev, Event::Html(_) | Event::InlineHtml(_)))`.
Confidence: medium
