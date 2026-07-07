# RUST_ISSUE_045: route renderers silently drop match criteria the translator recorded: Terraform `render_simple_route` omits `host_match`/`method_match` entirely, and both Terraform and JSON redirect/direct-response route renderers emit only the path criterion

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/f5-xc/src/terraform.rs:122-178 and rust/f5-xc/src/json_api.rs:165-199` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/f5-xc/src/terraform.rs:122-178 and rust/f5-xc/src/json_api.rs:165-199 — route renderers silently drop match criteria the translator recorded: Terraform `render_simple_route` omits `host_match`/`method_match` entirely, and both Terraform and JSON redirect/direct-response route renderers emit only the path criterion.
`if {[HTTP::host] eq "old.example.com"} { HTTP::redirect "https://new" }` produces a redirect route whose emitted TF/JSON has no host condition — every request gets redirected; `if {[HTTP::method] eq "POST"} { pool p }` loses the POST restriction in TF. JSON `render_simple_route` does emit `host`, proving the TF omission is unintended. Quote (terraform.rs redirect): only `if let Some(pm) = &route.path_match { lines.push(path_match_block(pm, level + 1)); }` before the action.
Confidence: high
