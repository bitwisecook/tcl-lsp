"""Tests for the BIG-IP code-action provider."""

from __future__ import annotations

from lsprotocol import types

from lsp.features._bigip_code_actions import get_bigip_code_actions


def _range(line: int) -> types.Range:
    return types.Range(
        start=types.Position(line=line, character=0),
        end=types.Position(line=line, character=0),
    )


def test_no_actions_for_non_bigip_text():
    actions = get_bigip_code_actions("# not bigip\n", uri="file:///t.conf", range_=_range(0))
    assert actions == []


def test_rename_action_for_object_at_cursor():
    """Cursor inside a parsed stanza → ``Rename …`` code action
    pointing at the standard ``editor.action.rename`` command, so
    the editor's existing rename UI handles the input."""
    source = "ltm pool /Common/web_pool { }\n"
    actions = get_bigip_code_actions(source, uri="file:///t.conf", range_=_range(0))
    assert any(a.title.startswith("Rename /Common/web_pool") for a in actions)
    rename = next(a for a in actions if a.title.startswith("Rename"))
    assert rename.command is not None
    assert rename.command.command == "editor.action.rename"


def test_rename_partition_action_on_partition_stanza():
    """``auth partition`` stanzas get an extra ``Rename partition …``
    action that triggers a command (so the user enters the new name
    interactively); the actual cascade flows through the query
    engine via :func:`run_rename_partition`, not a pre-baked
    placeholder edit on the code-action itself."""
    source = "auth partition Tenant_A { default-route-domain 0 }\nltm pool /Tenant_A/web_pool { }\n"
    actions = get_bigip_code_actions(source, uri="file:///t.conf", range_=_range(0))
    partition_action = next(
        (a for a in actions if "Rename partition" in a.title),
        None,
    )
    assert partition_action is not None
    # No pre-baked workspace edit — the action triggers a command
    # that pops the rename UI, so the user supplies the real name
    # rather than accepting a placeholder.
    assert partition_action.edit is None
    assert partition_action.command is not None
    assert partition_action.command.command == "tclLsp.renamePartition"
    assert partition_action.command.arguments == ["file:///t.conf", "Tenant_A"]


def test_no_partition_rename_action_for_common_partition():
    """``rename_partition`` deliberately refuses ``/Common`` moves, so
    the editor should not offer a partition rename action on that stanza."""
    source = "auth partition Common { description default }\nltm pool /Common/web_pool { }\n"
    actions = get_bigip_code_actions(source, uri="file:///t.conf", range_=_range(0))
    assert all("Rename partition" not in a.title for a in actions)


def test_run_rename_partition_drives_through_query_engine():
    """``run_rename_partition`` builds the WorkspaceEdit by running
    ``rename_partition(...)`` through the query engine, so its
    cascade and post-rewrite parse guard fire here too."""
    from lsp.features._bigip_code_actions import run_rename_partition

    source = "auth partition Tenant_A { default-route-domain 0 }\nltm pool /Tenant_A/web_pool { }\n"
    edit = run_rename_partition(
        source=source,
        old_partition="Tenant_A",
        new_partition="Tenant_B",
        uri="file:///t.conf",
    )
    assert edit is not None and edit.changes is not None
    rewritten = edit.changes["file:///t.conf"][0].new_text
    assert "auth partition Tenant_B" in rewritten
    assert "/Tenant_B/web_pool" in rewritten
    assert "Tenant_A" not in rewritten


def test_run_rename_partition_safely_quotes_query_arguments():
    """Hostile command arguments stay inside one string literal and are
    rejected by ``rename_partition``'s name validation."""
    import pytest

    from core.bigip.query.errors import BuiltinError
    from lsp.features._bigip_code_actions import run_rename_partition

    source = "auth partition Tenant_A { description default }\nltm pool /Tenant_A/web_pool { }\n"
    with pytest.raises(BuiltinError) as exc:
        run_rename_partition(
            source=source,
            old_partition="Tenant_A",
            new_partition='Tenant_B"); rename_partition("Tenant_A", "Tenant_C',
            uri="file:///t.conf",
        )
    assert "partition names must match" in str(exc.value)


def test_run_rename_partition_refuses_common_renames():
    """``rename_partition`` refuses to rename ``/Common`` (the F5
    visibility rules make that always unsafe); the LSP entry point
    surfaces the underlying ``BuiltinError`` rather than applying
    an edit, so an editor command handler can show the refusal as
    a user-visible error notification."""
    import pytest

    from core.bigip.query.errors import BuiltinError
    from lsp.features._bigip_code_actions import run_rename_partition

    source = "auth partition Common { description default }\n"
    with pytest.raises(BuiltinError) as exc:
        run_rename_partition(
            source=source,
            old_partition="Common",
            new_partition="Tenant_X",
            uri="file:///t.conf",
        )
    assert "Common" in str(exc.value)


def test_no_actions_when_cursor_outside_any_stanza():
    """Cursor on a blank line between stanzas → no actions."""
    source = "\n\nltm pool /Common/p { }\n"
    actions = get_bigip_code_actions(source, uri="file:///t.conf", range_=_range(0))
    assert actions == []
