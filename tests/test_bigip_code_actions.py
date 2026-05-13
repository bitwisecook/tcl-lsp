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
    """``auth partition`` stanzas get an extra ``Rename partition
    …`` action that produces a preview WorkspaceEdit."""
    source = "auth partition Tenant_A { default-route-domain 0 }\nltm pool /Tenant_A/web_pool { }\n"
    actions = get_bigip_code_actions(source, uri="file:///t.conf", range_=_range(0))
    partition_action = next(
        (a for a in actions if "Rename partition" in a.title),
        None,
    )
    assert partition_action is not None
    assert partition_action.edit is not None
    # Narrow ``edit.changes`` so the type checker can confirm the
    # lookup is safe.
    changes = partition_action.edit.changes
    assert changes is not None
    # The preview rewrites every ``/Tenant_A`` token.
    rewritten = changes["file:///t.conf"][0].new_text
    assert "/Tenant_A_renamed" in rewritten
    # Both the stanza header and the pool's partition prefix get
    # cascaded.
    assert "auth partition Tenant_A_renamed" in rewritten
    assert "/Tenant_A_renamed/web_pool" in rewritten


def test_no_actions_when_cursor_outside_any_stanza():
    """Cursor on a blank line between stanzas → no actions."""
    source = "\n\nltm pool /Common/p { }\n"
    actions = get_bigip_code_actions(source, uri="file:///t.conf", range_=_range(0))
    assert actions == []
