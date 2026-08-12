# KCS: feature — Claude Code Skills

> **Audience:** User
> **Type:** Functionality

## Summary

26 slash-command skills for Claude Code providing iRules, Tcl, Tk, and BIG-IP development assistance.

## Applies to

Claude skill

## Availability

| Context | How |
|---------|-----|
| Claude Code CLI | Type the skill name as a slash command |
| Claude Code Web | Type the skill name as a slash command |

## How to use

In Claude Code, type the skill name:

### iRules skills

| Skill | Description |
|-------|-------------|
| `/irule-create` | Create a new iRule from a description, validate, and iterate |
| `/irule-explain` | Explain what an iRule does with LSP context |
| `/irule-fix` | Fix issues using LSP diagnostics |
| `/irule-validate` | Run LSP diagnostics on an iRule |
| `/irule-review` | Security and safety review |
| `/irule-convert` | Modernise legacy patterns |
| `/irule-optimise` | Apply LSP optimisation suggestions |
| `/irule-scaffold` | Generate an iRule skeleton from events |
| `/irule-dataflow` | Analyse and visualise data flow in an iRule (def-use chains, Mermaid diagram) |
| `/irule-datagroup` | Suggest data-group extraction |
| `/irule-diff` | Explain differences between two versions |
| `/irule-event` | Show valid commands for an event |
| `/irule-diagram` | Generate a Mermaid flowchart |
| `/irule-migrate` | Convert nginx/Apache/HAProxy config to an iRule |
| `/irule-xc` | Translate to F5 XC configuration |

### Tcl skills

| Skill | Description |
|-------|-------------|
| `/tcl-create` | Create Tcl code from a description |
| `/tcl-explain` | Explain what a Tcl script does |
| `/tcl-fix` | Fix issues using LSP diagnostics |
| `/tcl-validate` | Run LSP diagnostics |
| `/tcl-optimise` | Apply LSP optimisation suggestions |
| `/tcl-refactor` | Apply mechanical refactorings (extract/inline variable, if-to-switch, switch-to-dict, brace expr) |

### BIG-IP skills

| Skill | Description |
|-------|-------------|
| `/bigip-cleanup` | Generate a tmsh delete script for unreferenced BIG-IP objects |
| `/explain-flow` | Trace a PCAP session through a BIG-IP config and explain what the device did |
| `/f5-query` | Translate a natural-language question into an `f5 query` DSL expression and run it |

### Tk skills

| Skill | Description |
|-------|-------------|
| `/tk-create` | Create a Tk GUI application |

### Meta

| Skill | Description |
|-------|-------------|
| `/ai-help` | Show available features and how to use them |

## Operational context

Skills call the native `tcl-mcp` MCP tools for analysis, then use AI to interpret results and generate code. The agentic create/fix skills iterate until LSP diagnostics are clean.

## Failure modes

- The `tcl-mcp` binary cannot be found or built (see `scripts/tcl-mcp`).
- Skills not loaded (check `.claude/` configuration).

## Test anchors

- Manual testing via Claude Code sessions.

## Example

In a Claude Code session with the `/irule-create` skill:

> `/irule-create an iRule that rewrites incoming /api requests to /v2/api`

Claude returns a draft iRule, then loops through `/irule-validate`
until all LSP diagnostics are clean:

```tcl
when HTTP_REQUEST {
    if {[HTTP::uri] starts_with "/api"} {
        HTTP::uri [string map {"/api" "/v2/api"} [HTTP::uri]]
    }
}
```

Follow-up commands such as `/irule-review` or `/irule-optimise`
operate on the same draft without you having to copy it back.

## Discoverability

- [KCS feature index](README.md)
