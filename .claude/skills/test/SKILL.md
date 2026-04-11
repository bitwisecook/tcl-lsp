---
name: test
description: >
  Run tests and report results. Delegates to a Sonnet agent so only
  failures, errors, and summary statistics enter the main context —
  keeping Opus token usage low on routine test runs.
allowed-tools: Agent
---

# Run Tests

Run a test target and return a concise summary. The heavy lifting (running
the command and parsing its output) is always delegated to a **Sonnet agent**
so that verbose test output never enters the main Opus context.

## How to execute

1. Parse `$ARGUMENTS` to determine the make target. Use the mapping below;
   default to `test-py` when no argument is given.
2. Spawn a **single** Agent with `model: "sonnet"` using the prompt template
   below. Do **not** run the test command yourself.
3. Relay the agent's summary to the user verbatim — do not embellish or
   re-summarise.

## Argument → make target mapping

| Argument | Make target | Notes |
|---|---|---|
| *(none)* | `make test-py` | Fast Python tests (default) |
| `py` | `make test-py` | Python tests |
| `all` | `make test` | Python + VS Code extension tests |
| `ext` | `make test-ext` | VS Code extension integration tests |
| `vm` | `make test-vm` | VM tcltest suite (slow) |
| `fuzz` | `make test-fuzz` | Differential fuzz tests |
| `slow` | `make test-slow` | Slow tests (extension + smoke) |
| `opt` | `make test-opt` | Optimiser coverage tests |
| `lint` | `make lint` | All lint and style checks |
| `typecheck` | `make typecheck-py` | Python type-checking with ty |
| `prep-pr` | `make prep-pr` | Full pre-PR gate (format + lint + typecheck + tests) |
| `rust` | `make rust-test` | Rust workspace tests |
| `rust-lint` | `make rust-lint` | Rust fmt check + clippy |
| `tclpkg` | `make test-tclpkg` | tclpkg package manager tests |
| `coverage` | `make coverage-py` | Python tests with coverage |
| *path* | `uv run pytest <path> -q` | Run specific test file or directory |

If the argument looks like a file path (contains `/` or ends in `.py`),
treat it as a pytest path argument instead of a make target.

## Sonnet agent prompt template

Use this prompt, substituting `{COMMAND}` with the resolved command:

~~~
You are a test-output analyst. Run the following command and produce a
concise summary of the results. Your working directory is the project root.

Command:
```bash
{COMMAND}
```

Run the command with a 10 minute timeout. Then analyse the output and
report ONLY the following:

1. **Result**: PASS or FAIL (one word)
2. **Stats**: total tests, passed, failed, skipped, errors, warnings
   (omit categories that are zero)
3. **Failures** (if any): for each failure list:
   - Test name / identifier
   - File path and line number
   - One-line cause (assertion message or exception)
   - The key 3-5 lines of the traceback (not the full trace)
4. **Errors** (if any): for each error list:
   - Error type and message
   - File path and line number
   - Brief context
5. **Warnings** (if any): list only novel or actionable warnings, not
   standard deprecation noise

Do NOT include:
- The raw command output
- Passing test names
- Timing information for individual tests
- Import/collection output
- Progress dots or percentage bars

Keep your entire response under 300 words for a passing run, or under
800 words for a failing run. Use markdown formatting.
~~~

$ARGUMENTS
