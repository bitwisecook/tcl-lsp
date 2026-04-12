"""Tests for BIG-IP linked object graph extraction."""

from __future__ import annotations

import textwrap

from core.bigip.link_extract import extract_linked_bigip_objects
from core.bigip.parser import parse_bigip_conf


def _node_by_header_prefix(result: dict, prefix: str) -> dict:
    for node in result["nodes"]:
        if node["header"].startswith(prefix):
            return node
    raise AssertionError(f"missing node with header prefix: {prefix!r}")


def test_extract_linked_objects_traverses_incoming_and_outgoing_links() -> None:
    source = textwrap.dedent(
        """\
        ltm node /Common/web1 {
            address 10.0.0.11
        }
        ltm pool /Common/web_pool {
            members {
                /Common/web1:80 { }
            }
            monitor /Common/http
        }
        ltm profile http /Common/custom_http {
            defaults-from /Common/http
        }
        ltm rule /Common/r1 {
            when HTTP_REQUEST {
                pool /Common/web_pool
            }
        }
        ltm virtual /Common/www_vs {
            destination /Common/192.0.2.10:443
            pool /Common/web_pool
            profiles {
                /Common/custom_http { }
            }
            rules {
                /Common/r1
            }
        }
        """
    )
    uri = "file:///bigip.conf"
    cfg = parse_bigip_conf(source)

    offset = source.index("ltm pool /Common/web_pool") + 5
    result = extract_linked_bigip_objects(
        uri=uri,
        offset=offset,
        sources={uri: source},
        configs={uri: cfg},
        max_depth=4,
    )

    assert result is not None
    pool = _node_by_header_prefix(result, "ltm pool /Common/web_pool")
    vs = _node_by_header_prefix(result, "ltm virtual /Common/www_vs")
    profile = _node_by_header_prefix(result, "ltm profile http /Common/custom_http")
    rule = _node_by_header_prefix(result, "ltm rule /Common/r1")

    assert pool["id"] == result["root"]
    assert vs["depth"] >= 1
    assert profile["depth"] >= 2
    assert rule["depth"] >= 1

    edge_pairs = {(edge["source"], edge["target"]) for edge in result["edges"]}
    assert (vs["id"], pool["id"]) in edge_pairs
    assert (vs["id"], profile["id"]) in edge_pairs
    assert (rule["id"], pool["id"]) in edge_pairs


def test_extract_linked_objects_resolves_across_bigip_conf_files() -> None:
    base_source = textwrap.dedent(
        """\
        ltm pool /Common/web_pool {
            members {
                /Common/web1:80 { }
            }
        }
        """
    )
    app_source = textwrap.dedent(
        """\
        ltm virtual /Common/www_vs {
            destination /Common/192.0.2.10:443
            pool /Common/web_pool
        }
        """
    )
    base_uri = "file:///bigip_base.conf"
    app_uri = "file:///bigip.conf"

    base_cfg = parse_bigip_conf(base_source)
    app_cfg = parse_bigip_conf(app_source)

    offset = base_source.index("ltm pool /Common/web_pool") + 8
    result = extract_linked_bigip_objects(
        uri=base_uri,
        offset=offset,
        sources={
            base_uri: base_source,
            app_uri: app_source,
        },
        configs={
            base_uri: base_cfg,
            app_uri: app_cfg,
        },
        max_depth=3,
    )

    assert result is not None
    pool = _node_by_header_prefix(result, "ltm pool /Common/web_pool")
    vs = _node_by_header_prefix(result, "ltm virtual /Common/www_vs")
    assert pool["uri"] == base_uri
    assert vs["uri"] == app_uri

    edge_pairs = {(edge["source"], edge["target"]) for edge in result["edges"]}
    assert (vs["id"], pool["id"]) in edge_pairs


def test_extract_linked_objects_keeps_gtm_rule_pool_links_in_gtm_domain() -> None:
    source = textwrap.dedent(
        """\
        ltm pool /Common/app_pool {
            members {
                /Common/10.0.0.11:80 { }
            }
        }
        gtm pool a /Common/app_pool { }
        gtm rule /Common/geo_rule {
            when DNS_REQUEST {
                pool /Common/app_pool
            }
        }
        """
    )
    uri = "file:///bigip_gtm.conf"
    cfg = parse_bigip_conf(source)
    offset = source.index("gtm rule /Common/geo_rule") + 5

    result = extract_linked_bigip_objects(
        uri=uri,
        offset=offset,
        sources={uri: source},
        configs={uri: cfg},
        max_depth=2,
    )

    assert result is not None
    rule = _node_by_header_prefix(result, "gtm rule /Common/geo_rule")
    gtm_pool = _node_by_header_prefix(result, "gtm pool a /Common/app_pool")
    edge_pairs = {(edge["source"], edge["target"]) for edge in result["edges"]}
    rule_edges = [edge for edge in result["edges"] if edge["source"] == rule["id"]]

    assert (rule["id"], gtm_pool["id"]) in edge_pairs
    assert not any(edge["viaKind"] == "ltm_pool" for edge in rule_edges)


def test_extract_linked_objects_multi_cursor_merges_graphs() -> None:
    """Multiple cursor offsets seed the BFS from separate roots."""
    source = textwrap.dedent(
        """\
        ltm pool /Common/pool_a {
            members {
                /Common/10.0.0.1:80 { }
            }
        }
        ltm pool /Common/pool_b {
            members {
                /Common/10.0.0.2:80 { }
            }
        }
        ltm rule /Common/irule_d {
            when HTTP_REQUEST {
                pool /Common/pool_a
            }
        }
        ltm virtual /Common/vs_a {
            pool /Common/pool_a
            rules {
                /Common/irule_d
            }
        }
        ltm virtual /Common/vs_b {
            pool /Common/pool_b
        }
        """
    )
    uri = "file:///bigip.conf"
    cfg = parse_bigip_conf(source)

    # Cursors in vs_a and irule_d – their combined graphs should cover both.
    offset_vs = source.index("ltm virtual /Common/vs_a") + 5
    offset_irule = source.index("ltm rule /Common/irule_d") + 5

    result = extract_linked_bigip_objects(
        offsets=[(uri, offset_vs), (uri, offset_irule)],
        sources={uri: source},
        configs={uri: cfg},
        max_depth=4,
    )

    assert result is not None

    # Both roots should appear.
    root_ids = {r["id"] for r in result["roots"]}
    assert len(root_ids) == 2

    headers = {node["header"] for node in result["nodes"]}
    assert "ltm virtual /Common/vs_a" in headers
    assert "ltm rule /Common/irule_d" in headers
    assert "ltm pool /Common/pool_a" in headers

    # Backward compat: root/rootUri/rootHeader point to the first seed.
    assert result["root"] == result["roots"][0]["id"]


def test_extract_linked_objects_multi_cursor_deduplicates_same_object() -> None:
    """Two cursors inside the same object produce a single root."""
    source = textwrap.dedent(
        """\
        ltm pool /Common/my_pool {
            members {
                /Common/10.0.0.1:80 { }
            }
        }
        """
    )
    uri = "file:///bigip.conf"
    cfg = parse_bigip_conf(source)

    offset_a = source.index("ltm pool") + 5
    offset_b = source.index("members") + 2

    result = extract_linked_bigip_objects(
        offsets=[(uri, offset_a), (uri, offset_b)],
        sources={uri: source},
        configs={uri: cfg},
    )

    assert result is not None
    assert len(result["roots"]) == 1


def test_extract_linked_objects_multi_cursor_across_files() -> None:
    """Cursors in different config files produce a merged graph."""
    base_source = textwrap.dedent(
        """\
        ltm pool /Common/web_pool {
            members {
                /Common/web1:80 { }
            }
        }
        """
    )
    app_source = textwrap.dedent(
        """\
        ltm virtual /Common/www_vs {
            destination /Common/192.0.2.10:443
            pool /Common/web_pool
        }
        """
    )
    base_uri = "file:///bigip_base.conf"
    app_uri = "file:///bigip.conf"

    base_cfg = parse_bigip_conf(base_source)
    app_cfg = parse_bigip_conf(app_source)

    result = extract_linked_bigip_objects(
        offsets=[
            (base_uri, base_source.index("ltm pool") + 5),
            (app_uri, app_source.index("ltm virtual") + 5),
        ],
        sources={base_uri: base_source, app_uri: app_source},
        configs={base_uri: base_cfg, app_uri: app_cfg},
        max_depth=3,
    )

    assert result is not None
    assert len(result["roots"]) == 2
    root_uris = {r["uri"] for r in result["roots"]}
    assert base_uri in root_uris
    assert app_uri in root_uris


def test_extract_linked_objects_source_origin_annotation() -> None:
    """Nodes carry a sourceOrigin tag derived from their URI filename."""
    base_source = textwrap.dedent(
        """\
        ltm pool /Common/web_pool {
            members {
                /Common/web1:80 { }
            }
        }
        """
    )
    app_source = textwrap.dedent(
        """\
        ltm virtual /Common/www_vs {
            pool /Common/web_pool
        }
        """
    )
    script_source = textwrap.dedent(
        """\
        ltm rule /Common/my_rule {
            when HTTP_REQUEST {
                pool /Common/web_pool
            }
        }
        """
    )
    base_uri = "file:///config/bigip_base.conf"
    app_uri = "file:///config/bigip.conf"
    script_uri = "file:///config/bigip_script.conf"

    base_cfg = parse_bigip_conf(base_source)
    app_cfg = parse_bigip_conf(app_source)
    script_cfg = parse_bigip_conf(script_source)

    result = extract_linked_bigip_objects(
        offsets=[
            (app_uri, app_source.index("ltm virtual") + 5),
        ],
        sources={
            base_uri: base_source,
            app_uri: app_source,
            script_uri: script_source,
        },
        configs={
            base_uri: base_cfg,
            app_uri: app_cfg,
            script_uri: script_cfg,
        },
        max_depth=4,
    )

    assert result is not None

    origin_by_header: dict[str, str | None] = {}
    for node in result["nodes"]:
        origin_by_header[node["header"]] = node["sourceOrigin"]

    assert origin_by_header["ltm virtual /Common/www_vs"] == "synced"
    assert origin_by_header["ltm pool /Common/web_pool"] == "base"
    assert origin_by_header.get("ltm rule /Common/my_rule") == "script"
