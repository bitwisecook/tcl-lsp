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

from lsprotocol import types

from core.bigip.parser import parse_bigip_conf


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
    # ``auth partition`` stanza.  Cascades the prefix rewrite via
    # the same engine ``f5 query 'rename_partition(...)'`` uses.
    # ``BigipAuthPartition.full_path`` is stored as the bare name
    # (``"Tenant_A"``), not with a leading slash, so the check
    # below matches the stanza identity rather than path shape.
    for part_path, part_obj in config.auth_partitions.items():
        if part_path != obj_path:
            continue
        rng = getattr(part_obj, "range", None)
        if rng is None or not (rng.start.line <= cursor_line <= rng.end.line):
            continue
        partition_short = obj_path.lstrip("/")
        renamed_short = f"{partition_short}_renamed"
        preview = _build_rename_partition_preview(
            source,
            old_partition=partition_short,
            new_partition=renamed_short,
            uri=uri,
        )
        if preview is not None:
            actions.append(
                types.CodeAction(
                    title=(
                        f"Rename partition {partition_short!r} → "
                        f"{renamed_short!r} (preview only — adjust in DSL)"
                    ),
                    kind=types.CodeActionKind.RefactorRewrite,
                    edit=preview,
                )
            )

    return actions


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


def _build_rename_partition_preview(
    source: str,
    *,
    old_partition: str,
    new_partition: str,
    uri: str,
) -> types.WorkspaceEdit | None:
    """Construct a workspace edit cascading the full partition rename.

    Drives the same two token-bounded regex rewrites the
    ``rename_partition(old, new)`` query DSL builtin uses:

    - ``/Old/...`` → ``/New/...`` for every path-prefixed reference
      (object headers, refs in compound values, iRule body
      literals).  Token-bounded so ``/OldExt/...`` doesn't match.
    - ``auth partition Old { ... }`` → ``auth partition New { ... }``
      for the partition stanza header itself.

    Then post-rewrite-parses the result to confirm the cascade
    produced valid SCF — same defence-in-depth the
    ``rename_partition`` builtin gets via the edit-plan applier.
    Returns ``None`` if the rewrite produced 0 occurrences or
    invalid SCF.
    """
    import re

    prefix_pattern = re.compile(
        rf"(?<![A-Za-z0-9_/.\-])/{re.escape(old_partition)}/(?=[A-Za-z0-9_])"
    )
    header_pattern = re.compile(
        rf"(?<![A-Za-z0-9_/.\-])(auth\s+partition\s+){re.escape(old_partition)}"
        rf"(?![A-Za-z0-9_/.\-])"
    )
    new_source, prefix_n = prefix_pattern.subn(f"/{new_partition}/", source)
    new_source, header_n = header_pattern.subn(rf"\g<1>{new_partition}", new_source)
    if prefix_n + header_n == 0:
        return None
    # Defence-in-depth: refuse the preview when it doesn't reparse.
    try:
        parse_bigip_conf(new_source)
    except Exception:  # noqa: BLE001
        return None
    lines = source.split("\n")
    last_line = len(lines) - 1
    full_range = types.Range(
        start=types.Position(line=0, character=0),
        end=types.Position(line=last_line, character=len(lines[-1])),
    )
    return types.WorkspaceEdit(
        changes={uri: [types.TextEdit(range=full_range, new_text=new_source)]},
    )
