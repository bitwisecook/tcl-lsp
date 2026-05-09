"""Tests for BIG-IP unreferenced-object cleanup analysis."""

from __future__ import annotations

import textwrap
from pathlib import Path

from core.bigip.cleanup import compute_cleanup, report_to_dict
from core.bigip.parser import parse_bigip_conf
from core.commands.registry.runtime import active_signature_profile


def _run(source: str, **kwargs):
    cfg = parse_bigip_conf(source)
    uri = "mem://test.conf"
    return compute_cleanup(sources={uri: source}, configs={uri: cfg}, **kwargs)


def _delete_lines(report) -> list[str]:
    return [c.delete_command for c in report.candidates]


def _full_paths(report) -> list[str]:
    return [c.full_path for c in report.candidates]


def test_fully_referenced_config_yields_no_candidates() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm monitor http /Common/m1 { defaults-from /Common/http }
        ltm pool /Common/p1 {
            members { /Common/n1:80 { address 10.0.0.1 } }
            monitor /Common/m1
        }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            pool /Common/p1
        }
        """
    )
    report = _run(source, keep_partitions=frozenset())
    assert report.candidates == ()
    assert report.summary == {}
    assert "Candidates: none" in report.tmsh_script


def test_orphan_pool_chain_orders_pool_then_node_then_monitor() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n_orphan { address 10.0.0.99 }
        ltm monitor http /Common/m_orphan { defaults-from /Common/http }
        ltm pool /Common/p_orphan {
            members { /Common/n_orphan:80 { address 10.0.0.99 } }
            monitor /Common/m_orphan
        }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
        }
        """
    )
    report = _run(source, keep_partitions=frozenset())
    paths = _full_paths(report)

    assert set(paths) == {"/Common/n_orphan", "/Common/m_orphan", "/Common/p_orphan"}
    # Pool releases its references before node/monitor become deletable.
    assert paths.index("/Common/p_orphan") < paths.index("/Common/n_orphan")
    assert paths.index("/Common/p_orphan") < paths.index("/Common/m_orphan")


def test_irule_object_references_keep_data_group_and_pool_alive() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/used_pool {
            members { /Common/n1:80 { address 10.0.0.1 } }
        }
        ltm data-group internal /Common/used_dg {
            type string
            records { x { data y } }
        }
        ltm rule /Common/r1 {
        when HTTP_REQUEST {
            pool /Common/used_pool
            if { [class match [HTTP::host] equals /Common/used_dg] } { reject }
        }
        }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
            rules { /Common/r1 }
        }
        """
    )
    report = _run(source, keep_partitions=frozenset())
    assert report.candidates == ()


def test_orphan_irule_chain_deletes_rule_before_data_group_and_pool() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/n_orphan { address 10.0.0.99 }
        ltm pool /Common/p_orphan {
            members { /Common/n_orphan:80 { address 10.0.0.99 } }
        }
        ltm data-group internal /Common/dg_orphan {
            type string
            records { x { data y } }
        }
        ltm rule /Common/r_orphan {
        when HTTP_REQUEST {
            pool /Common/p_orphan
            if { [class match [HTTP::host] equals /Common/dg_orphan] } { reject }
        }
        }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
        }
        """
    )
    report = _run(source, keep_partitions=frozenset())
    paths = _full_paths(report)

    assert set(paths) == {
        "/Common/r_orphan",
        "/Common/p_orphan",
        "/Common/n_orphan",
        "/Common/dg_orphan",
    }
    # Rule references both pool and dg, so it must be deleted first to release them.
    assert paths.index("/Common/r_orphan") < paths.index("/Common/p_orphan")
    assert paths.index("/Common/r_orphan") < paths.index("/Common/dg_orphan")
    # Pool releases its node membership before the node can be removed.
    assert paths.index("/Common/p_orphan") < paths.index("/Common/n_orphan")


def test_keep_paths_protect_specific_objects() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/keep_me { address 10.1.1.1 }
        ltm node /Common/drop_me { address 10.2.2.2 }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
        }
        """
    )
    report = _run(
        source,
        keep_paths=frozenset({"/Common/keep_me"}),
        keep_partitions=frozenset(),
    )
    assert _full_paths(report) == ["/Common/drop_me"]


def test_default_common_partition_filter_keeps_factory_objects() -> None:
    """By default, /Common/*-prefixed orphans are kept (factory objects)."""
    source = textwrap.dedent(
        """\
        ltm node /Common/factory_node { address 10.1.1.1 }
        ltm node /Site/orphan_node { address 10.2.2.2 }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
        }
        """
    )
    report = _run(source)
    assert _full_paths(report) == ["/Site/orphan_node"]


def test_no_keep_common_includes_factory_objects() -> None:
    """Passing an empty keep_partitions set deletes /Common/* orphans too."""
    source = textwrap.dedent(
        """\
        ltm node /Common/factory_node { address 10.1.1.1 }
        ltm node /Site/orphan_node { address 10.2.2.2 }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
        }
        """
    )
    report = _run(source, keep_partitions=frozenset())
    assert set(_full_paths(report)) == {"/Common/factory_node", "/Site/orphan_node"}


def test_virtual_servers_are_never_candidates_even_when_only_root() -> None:
    """Every virtual is a graph root; cleanup never proposes deleting one."""
    source = textwrap.dedent(
        """\
        ltm virtual /Common/vs_orphan {
            destination /Common/10.0.0.10:80
        }
        """
    )
    report = _run(source, keep_partitions=frozenset())
    assert report.candidates == ()


def test_tmsh_script_uses_module_and_object_type_verbatim() -> None:
    """The delete command must include the multi-word object type (e.g. ``monitor http``)."""
    source = textwrap.dedent(
        """\
        ltm monitor http /Site/m_orphan { defaults-from /Common/http }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
        }
        """
    )
    report = _run(source, keep_partitions=frozenset())
    assert _delete_lines(report) == ["delete ltm monitor http /Site/m_orphan"]


def test_report_to_dict_round_trip_is_json_serialisable() -> None:
    import json

    source = textwrap.dedent(
        """\
        ltm node /Site/n_orphan { address 10.0.0.99 }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
        }
        """
    )
    report = _run(source, keep_partitions=frozenset())
    payload = report_to_dict(report)
    raw = json.dumps(payload)
    again = json.loads(raw)

    assert again["summary"] == {"ltm_node": 1}
    assert again["candidates"][0]["deleteCommand"] == "delete ltm node /Site/n_orphan"
    assert again["tmshScript"].startswith("# tcl-lsp BIG-IP cleanup")


def test_compute_cleanup_restores_signature_profile() -> None:
    """compute_cleanup must not leak the f5-irules switch to the caller."""
    saved = active_signature_profile()
    source = textwrap.dedent(
        """\
        ltm node /Site/n_orphan { address 10.0.0.99 }
        ltm virtual /Common/vs {
            destination /Common/10.0.0.10:80
        }
        """
    )
    _run(source, keep_partitions=frozenset())
    assert active_signature_profile() == saved


def test_sample_bigip_conf_has_no_orphans() -> None:
    """Sanity check against the in-tree sample — every object is referenced."""
    sample = Path(__file__).parent.parent / "samples" / "bigip" / "bigip.conf"
    src = sample.read_text(encoding="utf-8")
    cfg = parse_bigip_conf(src)
    report = compute_cleanup(sources={sample.as_uri(): src}, configs={sample.as_uri(): cfg})
    assert report.candidates == ()
    assert len(report.roots) >= 1
