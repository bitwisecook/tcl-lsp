---
name: fuzz-findings
description: >
  Manage differential-fuzzer findings: query fixed/unfixed status, list by
  category, verify findings against the current VM, mark findings as
  fixed/unfixed, and batch-update categories. Use when working on fuzz
  findings, checking what's fixed, or triaging new results.
allowed-tools: Bash, Read, Write, Edit
---

# Fuzz Findings Management

Manages the differential-fuzzer findings in `tests/fuzz/findings/`.
Each finding is a JSON file recording mismatches between backends
(vm, vm_opt, tclsh) with an optional `.tcl` script that triggered it.

## Usage

```bash
python3 .claude/skills/fuzz-findings/fuzz_findings.py <command> [args...]
```

## Commands

| Command | Arguments | What it does |
|---|---|---|
| `summary` | | Overview: total/fixed/unfixed counts by category |
| `list` | `[--fixed\|--unfixed] [--category CAT] [--limit N]` | List findings filtered by status and category |
| `show` | `SEED` | Show full JSON + TCL script for a specific finding |
| `mark-fixed` | `SEED [--fix DESC] [--category CAT]` | Mark a finding as fixed |
| `mark-unfixed` | `SEED` | Reopen a finding (mark unfixed) |
| `categorize` | `SEED CATEGORY` | Set/change a finding's category |
| `verify` | `[--unfixed\|--fixed\|--all] [--category CAT] [--timeout S] [--limit N]` | Run findings through VM to check current status |
| `batch-mark` | `--category CAT --fix DESC [--timeout S]` | Auto-mark verified-fixed findings in a category |
| `run` | `SEED [--timeout S] [--optimise]` | Run a single finding's TCL through the VM |

## Typical workflows

### Check what's still broken
```bash
python3 .claude/skills/fuzz-findings/fuzz_findings.py summary
python3 .claude/skills/fuzz-findings/fuzz_findings.py verify --unfixed --category vm-timeout-float-int
```

### After fixing a VM bug, mark resolved findings
```bash
python3 .claude/skills/fuzz-findings/fuzz_findings.py batch-mark --category vm-timeout-float-int --fix "added float rejection in bitwise ops"
```

### Triage a new finding
```bash
python3 .claude/skills/fuzz-findings/fuzz_findings.py show 1772893252
python3 .claude/skills/fuzz-findings/fuzz_findings.py run 1772893252 --timeout 5
python3 .claude/skills/fuzz-findings/fuzz_findings.py categorize 1772893252 vm-timeout-float-int
```

### Verify no regressions in fixed findings
```bash
python3 .claude/skills/fuzz-findings/fuzz_findings.py verify --fixed
```

## Categories

Common categories used for triaging:

| Category | Description |
|---|---|
| `vm-timeout-undefined-var` | VM loops on scripts with undefined variables |
| `vm-timeout-float-int` | VM loops on float values in integer-only ops |
| `vm-timeout-invalid-cmd` | VM loops on invalid command names |
| `vm-timeout-type-error` | VM loops on type mismatches |
| `vm-timeout-negative-shift` | VM loops on negative shift arguments |
| `vm-timeout-if-else` | VM loops on if-else-elseif structure errors |
| `vm-timeout-other` | Miscellaneous timeout causes |
| `parse-divergence` | vm vs vm_opt parse corrupted input differently |
| `vm-too-strict` | VM rejects valid input that tclsh accepts |
| `timeout-all` | All backends timeout (not a bug) |

## Finding JSON schema

```json
{
  "seed": 1772822330,
  "mismatches": [
    {
      "kind": "crash | error_status | output",
      "description": "...",
      "backend_a": "vm | vm_opt | tclsh(tclsh9.0)",
      "backend_b": "vm | vm_opt | tclsh(tclsh9.0)",
      "detail_a": "error message | (ok) | TIMEOUT | CRASH: ...",
      "detail_b": "error message | (ok) | TIMEOUT | CRASH: ..."
    }
  ],
  "fixed": true,
  "fix": "short description of the fix",
  "category": "category-name"
}
```

$ARGUMENTS
