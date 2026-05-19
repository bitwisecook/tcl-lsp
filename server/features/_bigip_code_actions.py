"""BIG-IP code actions — quick refactors backed by the query engine.

The query DSL has all the engines for "rename partition", "delete
object", "set property", etc. (every mutating ``f5 query``
expression).  This module exposes a handful of those as
:class:`types.CodeAction` items so the editor can offer them as
"💡 quick-fix" entries when the user's cursor lands on a relevant
stanza — no need for the user to remember the DSL syntax for
common refactors.

Each action returns a :class:`WorkspaceEdit` so the editor can
preview the diff inline; no custom commands or callbacks needed
on the client side.

Actions provided in v1:

- On an ``auth partition X`` stanza: **Rename this partition…**
  proposes a placeholder name; user accepts → ``rename_partition``
  cascade across the file.
- On any ``ltm / gtm / net / sys / …`` stanza: **Rename this
  object…** drives the object-identity rename through
  :func:`rename_object`.

The recipe library can grow as v2; the v1 cut targets the two
recipes that lose the most time when done by hand (partition
rename touches dozens of objects; object rename touches every
reference).
"""

from __future__ import annotations

import json

from lsprotocol import types

from dialects.f5.bigip.parser import parse_bigip_conf


def get_bigip_code_actions(
    source: str,
    *,
    uri: str,
    range_: types.Range,
) -> list[types.CodeAction]:
    """Return BIG-IP-specific code actions for the cursor range.

    Walks the parsed config looking for an object whose stanza
    covers the cursor; emits action entries for that object.
    Empty list when the cursor isn't on a parseable stanza or
    the BIG-IP parser fails.
    """
    try:
        config = parse_bigip_conf(source)
    except Exception:  # noqa: BLE001
        return []

    cursor_line = range_.start.line
    actions: list[types.CodeAction] = []
    obj_path = _object_at_cursor(config, cursor_line)
    if obj_path is None:
        return actions

    # Rename-this-object action — produces a WorkspaceEdit by calling
    # rename_object with a placeholder.  The editor will preview the
    # diff; the user adjusts the placeholder name in the inline
    # rename UI.  For LSP, we emit the action as a Command pointing
    # at the standard ``textDocument/rename`` flow.
    actions.append(
        types.CodeAction(
            title=f"Rename {obj_path}…",
            kind=types.CodeActionKind.RefactorRewrite,
            command=types.Command(
                title="Rename",
                command="editor.action.rename",
                arguments=[uri, range_.start],
            ),
        )
    )

    # Partition rename action — only when the cursor is on an
    # ``auth partition`` stanza.  Routes through the query engine
    # (``rename_partition``) so the same partition-visibility
    # refusal rules the CLI uses fire here too.  Renames of
    # ``/Common`` are suppressed up front; renames into ``/Common``
    # from a tenant are rejected by the execute-command handler.
    # ``BigipAuthPartition.full_path`` is stored as the bare name
    # (``"Tenant_A"``) without a leading slash.
    for part_path, part_obj in config.auth_partitions.items():
        if part_path != obj_path:
            continue
        rng = getattr(part_obj, "range", None)
        if rng is None or not (rng.start.line <= cursor_line <= rng.end.line):
            continue
        partition_short = obj_path.lstrip("/")
        if partition_short == "Common":
            continue
        # Offer the action as a command that triggers the standard
        # rename UI rather than as a pre-baked WorkspaceEdit with a
        # placeholder name.  The previous behaviour applied a
        # ``<partition>_renamed`` workspace edit on accept, which
        # contradicted the "preview only" title and could land an
        # unintended placeholder on disk; this command opens the
        # rename dialog where the user supplies the real new name,
        # and the rename then flows through the same code path the
        # CLI uses for ``f5 query 'rename_partition(...)'``.
        actions.append(
            types.CodeAction(
                title=f"Rename partition {partition_short!r}…",
                kind=types.CodeActionKind.RefactorRewrite,
                command=types.Command(
                    title="Rename partition",
                    command="tclLsp.renamePartition",
                    arguments=[uri, partition_short],
                ),
            )
        )

    return actions


def run_rename_partition(
    *,
    source: str,
    old_partition: str,
    new_partition: str,
    uri: str,
) -> types.WorkspaceEdit | None:
    """Drive ``rename_partition(old, new)`` through the query engine.

    The shared entry point the editor command (and any future
    code action that wants to apply a partition rename) calls
    after collecting the new name from the user.  Returns
    ``None`` when:

    - the query engine refuses the rename (partition-visibility
      guards reject any rename involving ``/Common``);
    - the cascade produces zero matches (no-op);
    - the post-rewrite source fails to reparse (defence in
      depth — the same guard the CLI applies).

    On success, returns a single-file :class:`WorkspaceEdit`
    rewriting the full text so the editor previews the diff
    normally.  Partition-visibility refusal raises
    :class:`dialects.f5.query.errors.BuiltinError`, which the
    caller (LSP command handler) can surface as a user-visible
    error notification rather than a silent no-op.
    """
    from dialects.f5.query.errors import BuiltinError
    from dialects.f5.query.runner import run_query

    if old_partition == new_partition:
        return None
    expression = f"rename_partition({json.dumps(old_partition)}, {json.dumps(new_partition)})"
    try:
        result = run_query(expression, {uri: source})
    except BuiltinError:
        raise
    applied = result.edits_per_file.get(uri)
    if applied is None or applied.new_source == applied.original:
        return None
    try:
        parse_bigip_conf(applied.new_source)
    except Exception:  # noqa: BLE001
        return None
    lines = source.split("\n")
    last_line = len(lines) - 1
    full_range = types.Range(
        start=types.Position(line=0, character=0),
        end=types.Position(line=last_line, character=len(lines[-1])),
    )
    return types.WorkspaceEdit(
        changes={uri: [types.TextEdit(range=full_range, new_text=applied.new_source)]},
    )


def _object_at_cursor(config, cursor_line: int) -> str | None:
    """Return the full-path of the object whose stanza covers *cursor_line*.

    Walks every dict-valued field on the config and returns the
    first object whose ``range`` includes the cursor.  ``None``
    when the cursor isn't inside any known stanza.
    """
    from dataclasses import fields

    for fld in fields(config):
        if fld.name == "generic_objects":
            continue
        kind_dict = getattr(config, fld.name)
        if not isinstance(kind_dict, dict):
            continue
        for full_path, obj in kind_dict.items():
            rng = getattr(obj, "range", None)
            if rng is None:
                continue
            if rng.start.line <= cursor_line <= rng.end.line:
                return full_path
    return None
