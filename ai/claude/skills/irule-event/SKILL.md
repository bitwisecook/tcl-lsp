---
name: irule-event
description: >
  Look up iRules event or command reference. Shows which commands
  are valid in an event, or which events support a command.
  Uses authoritative registry metadata from the LSP.
allowed-tools: Bash, Read
---

# iRule Event/Command Reference

Look up event or command information from the iRules registry.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Determine what the user is asking about:
   - If they mention an event name (e.g. HTTP_REQUEST, CLIENT_ACCEPTED), look up event info
   - If they mention a command (e.g. HTTP::header, IP::client_addr), look up command info
   - If unclear, ask for clarification
3. Run the appropriate lookup:

   For events:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py event-info EVENT_NAME
   ```

   For commands:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py command-info COMMAND_NAME
   ```

4. Present the registry metadata (authoritative facts), then provide practical guidance:

   For events:
   - When it fires
   - Common commands to use
   - Available request/response data
   - Performance and safety notes
   - Minimal useful example

   For commands:
   - Syntax and options
   - Valid/invalid events
   - Typical usage patterns
   - Common mistakes

## Important

The registry metadata is authoritative — always present it as facts.
Supplement with practical guidance, but clearly separate facts from advice.

$ARGUMENTS
