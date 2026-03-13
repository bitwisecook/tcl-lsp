# Command Registry

Python-native command metadata for completion and hover.

## Layout

- `models.py`: dataclasses for command/form/option/value/hover snippets.
- `_base.py`: shared `CommandDef` base class and `make_av()` helper factory.
- `command_registry.py`: lookup API and global `REGISTRY`.
- `runtime.py`: registry runtime (dialects, roles, body/expr index helpers).
- `tcl/`: one class file per Tcl command with `@register` decorator.
- `irules/`: dialect-specific specs for F5 iRules.
- `iapps/`: F5 iApps utility command specs.

Command specs can include:
- `validation`: arity/subcommand metadata used for syntax checks.

## Adding Tcl Commands

1. Create a class file under `tcl/` following the `CommandDef` + `@register` pattern.
2. Scaffold initial files from Tcl man pages:
   - `python scripts/registry/scaffold_tcl_commands.py --doc-dir /path/to/tcl9/doc`
3. Add the import line to `tcl/__init__.py`.
4. Refine hover, options, arg_values, and role_hints as needed.

## Adding Dialect-Specific Commands

1. Add specs under `<dialect>/` (for example `irules/http.py`).
2. Set `dialects=frozenset({"<dialect-name>"})`.
3. Export from `<dialect>/__init__.py`.
4. Include in `command_registry.py` through `_all_command_specs()`.

## Regenerating iRules Baseline

1. Regenerate iRules metadata from F5 reference documentation using the generation tooling.
2. Keep curated command-level overrides in `irules/http.py` for higher-fidelity docs.

## Runtime Usage

- Completion:
  - `REGISTRY.switches(command, dialect)`
  - `REGISTRY.argument_values(command, arg_index, dialect)`
- Validation:
  - `REGISTRY.validation(command, dialect)` for arity/subcommand checks
  - `REGISTRY.command_names(dialect)` for command inventories
- Hover:
  - `REGISTRY.get(command, dialect)`
  - `REGISTRY.option(command, option, dialect)`
  - `REGISTRY.argument_value(command, arg_index, value, dialect)`
