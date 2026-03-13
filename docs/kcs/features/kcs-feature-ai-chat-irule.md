# KCS: feature — @irule Chat Participant

## Summary

VS Code Copilot Chat participant for creating, explaining, fixing, reviewing, and transforming F5 BIG-IP iRules.

## Surface

vscode-chat

## Availability

| Context | How |
|---------|-----|
| VS Code Copilot Chat | Type `@irule` then a slash command or question |

## How to use

Type `@irule` in the Copilot Chat panel followed by a slash command:

| Command | Description |
|---------|-------------|
| `/create` | Create a new iRule from a description |
| `/explain` | Explain what an iRule does |
| `/fix` | Fix issues found by the LSP |
| `/validate` | Run LSP diagnostics |
| `/review` | Security and safety review |
| `/convert` | Modernise legacy patterns (matchclass, unbraced expr) |
| `/optimise` | Apply LSP optimisations with explanations |
| `/scaffold` | Generate an iRule skeleton from events |
| `/datagroup` | Suggest data-group extraction for inline lookups |
| `/diff` | Compare two iRule versions |
| `/event` | Event and command reference |
| `/diagram` | Generate a Mermaid flowchart of the iRule logic |
| `/migrate` | Convert nginx/Apache/HAProxy config to an iRule |
| `/xc` | Translate to F5 XC routes and service policies |
| `/help` | Show available features and commands |

Or ask a free-form iRules question without a slash command.

## Operational context

The chat participant uses the LSP server for diagnostics, symbols, and optimisations, then sends the analysis context to the language model. The agentic loop can iteratively fix code until diagnostics are clean. Requires `tclLsp.ai.enabled` to be true.

## File-path anchors

- `editors/vscode/src/chat/iruleParticipant.ts`
- `editors/vscode/src/chat/commands/`
- `editors/vscode/src/chat/agenticLoop.ts`
- `editors/vscode/src/chat/contextPack.ts`

## Failure modes

- AI features disabled (`tclLsp.ai.enabled` is false).
- Copilot extension not installed.

## Test anchors

- `editors/vscode/src/test/chatUtilities.test.ts`

## Screenshots

- `26-ai-create` — @irule /create generating an iRule
- `27-ai-explain` — @irule /explain breaking down an iRule
- `28-ai-diagram` — @irule /diagram generating a Mermaid flowchart
- `29-ai-validate` — @irule /validate running LSP diagnostics
- `30-ai-review` — @irule /review security review
- `31-ai-help` — @irule /help showing feature guide

![@irule /create generating an iRule](../screenshots/26-ai-create.png)
![@irule /explain breaking down an iRule](../screenshots/27-ai-explain.png)
![@irule /diagram generating a Mermaid flowchart](../screenshots/28-ai-diagram.png)
![@irule /validate running LSP diagnostics](../screenshots/29-ai-validate.png)
![@irule /review security review](../screenshots/30-ai-review.png)
![@irule /help showing feature guide](../screenshots/31-ai-help.png)

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../kcs-vscode-extension-contracts.md)
