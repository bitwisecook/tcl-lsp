"""Tests for the BIG-IP cross-reference validator."""

from __future__ import annotations

from core.bigip.parser import parse_bigip_conf
from core.bigip.validator import validate_bigip_config


def _codes(diagnostics: list) -> list[str]:
    """Extract diagnostic codes from a list of diagnostics."""
    return sorted(d.code for d in diagnostics)


def _codes_set(diagnostics: list) -> set[str]:
    return {d.code for d in diagnostics}


def _msgs(diagnostics: list, code: str) -> list[str]:
    return [d.message for d in diagnostics if d.code == code]


# BIGIP6001: iRule references missing data-group


def test_bigip6001_missing_data_group():
    source = """\
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        if { [class match [HTTP::host] equals missing_dg] } {
            drop
        }
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6001" in _codes_set(diags)
    msgs = _msgs(diags, "BIGIP6001")
    assert any("missing_dg" in m for m in msgs)


def test_bigip6001_existing_data_group():
    source = """\
ltm data-group internal /Common/my_dg {
    records {
        test { }
    }
    type string
}
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        if { [class match [HTTP::host] equals my_dg] } {
            pool /Common/web_pool
        }
    }
}
ltm pool /Common/web_pool {
    members {
        /Common/node1:80 { }
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6001" not in _codes_set(diags)


# BIGIP6002: iRule references missing pool


def test_bigip6002_missing_pool():
    source = """\
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        pool /Common/nonexistent_pool
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6002" in _codes_set(diags)


def test_bigip6002_missing_pool_range_points_to_pool_token():
    source = """\
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        pool /Common/nonexistent_pool
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    pool_diag = next(d for d in diags if d.code == "BIGIP6002")

    rule_source = config.rules["/Common/my_rule"].source
    line_text = rule_source.splitlines()[pool_diag.range.start.line]
    token = line_text[pool_diag.range.start.character : pool_diag.range.end.character + 1]
    assert token == "/Common/nonexistent_pool"


def test_bigip6002_existing_pool():
    source = """\
ltm pool /Common/web_pool {
    members {
        /Common/node1:80 { }
    }
}
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        pool /Common/web_pool
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6002" not in _codes_set(diags)


def test_bigip6002_pool_short_name():
    source = """\
ltm pool /Common/web_pool {
    members {
        /Common/node1:80 { }
    }
}
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        pool web_pool
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6002" not in _codes_set(diags)


# BIGIP6003: Virtual references missing iRule


def test_bigip6003_missing_rule():
    source = """\
ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:80
    rules {
        /Common/nonexistent_rule
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6003" in _codes_set(diags)


def test_bigip6003_existing_rule():
    source = """\
ltm rule /Common/my_rule {
    when HTTP_REQUEST { }
}
ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:80
    rules {
        /Common/my_rule
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6003" not in _codes_set(diags)


# BIGIP6004: Profile requirements


def test_bigip6004_http_commands_without_http_profile():
    source = """\
ltm profile tcp /Common/my_tcp {
    defaults-from /Common/tcp
}
ltm rule /Common/my_rule {
    when CLIENT_ACCEPTED {
        log local0. "HTTP host: [HTTP::host]"
    }
}
ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:80
    profiles {
        /Common/my_tcp { }
    }
    rules {
        /Common/my_rule
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6004" in _codes_set(diags)
    msgs = _msgs(diags, "BIGIP6004")
    assert any("HTTP::" in m and "no HTTP profile" in m for m in msgs)


def test_bigip6004_http_commands_with_http_profile():
    source = """\
ltm profile http /Common/my_http {
    defaults-from /Common/http
}
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        log local0. "Host: [HTTP::host]"
    }
}
ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:80
    profiles {
        /Common/my_http { }
    }
    rules {
        /Common/my_rule
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    bigip6004_msgs = _msgs(diags, "BIGIP6004")
    assert not any("HTTP::" in m and "no HTTP profile" in m for m in bigip6004_msgs)


def test_bigip6004_ssl_commands_without_ssl_profile():
    source = """\
ltm profile tcp /Common/my_tcp {
    defaults-from /Common/tcp
}
ltm rule /Common/my_rule {
    when CLIENTSSL_HANDSHAKE {
        log local0. "Cipher: [SSL::cipher name]"
    }
}
ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:443
    profiles {
        /Common/my_tcp { }
    }
    rules {
        /Common/my_rule
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6004" in _codes_set(diags)
    msgs = _msgs(diags, "BIGIP6004")
    assert any("SSL::" in m and "no SSL profile" in m for m in msgs)


# BIGIP6005: Virtual references missing pool


def test_bigip6005_missing_vs_pool():
    source = """\
ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:80
    pool /Common/nonexistent_pool
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6005" in _codes_set(diags)


# BIGIP6006: Unused data-group


def test_bigip6006_unused_data_group():
    source = """\
ltm data-group internal /Common/unused_dg {
    records {
        test { }
    }
    type string
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6006" in _codes_set(diags)
    msgs = _msgs(diags, "BIGIP6006")
    assert any("unused_dg" in m for m in msgs)


def test_bigip6006_used_data_group():
    source = """\
ltm data-group internal /Common/my_dg {
    records {
        test { }
    }
    type string
}
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        class match [HTTP::host] equals my_dg
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6006" not in _codes_set(diags)


# BIGIP6007: iRule references missing SNAT pool


def test_bigip6007_missing_snatpool():
    source = """\
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        snatpool /Common/missing_sp
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6007" in _codes_set(diags)


# BIGIP6008: Empty pool


def test_bigip6008_empty_pool():
    source = """\
ltm pool /Common/empty_pool {
    monitor /Common/http
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6008" in _codes_set(diags)


def test_bigip6008_pool_with_members():
    source = """\
ltm pool /Common/web_pool {
    members {
        /Common/web1:80 { }
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6008" not in _codes_set(diags)


# BIGIP6009: Duplicate iRule attachment


def test_bigip6009_duplicate_rule():
    source = """\
ltm rule /Common/my_rule {
    when HTTP_REQUEST { }
}
ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:80
    rules {
        /Common/my_rule
        /Common/my_rule
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6009" in _codes_set(diags)


# Full example config


def test_validate_example_config():
    """Validate the example bigip.conf and check expected diagnostic codes."""
    import os

    conf_path = os.path.join(os.path.dirname(__file__), "..", "samples", "bigip", "bigip.conf")
    with open(conf_path) as f:
        source = f.read()
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    codes = _codes_set(diags)

    # The example config has intentional bad references
    assert "BIGIP6001" in codes  # nonexistent_datagroup
    assert "BIGIP6002" in codes  # missing_pool in irule
    assert "BIGIP6003" in codes  # nonexistent_rule on bad_vs
    assert "BIGIP6004" in codes  # HTTP:: commands on ssl_only_vs (no HTTP profile)
    assert "BIGIP6005" in codes  # nonexistent_pool on bad_vs
    assert "BIGIP6006" in codes  # unused_datagroup
    assert "BIGIP6007" in codes  # missing_snatpool in irule
    assert "BIGIP6008" in codes  # empty_pool
    assert "BIGIP6009" in codes  # duplicate bad_references on bad_vs


# Variable / command-substitution references are skipped


def test_skip_variable_references():
    source = """\
ltm rule /Common/my_rule {
    when HTTP_REQUEST {
        pool $pool_name
        class match [HTTP::host] equals $dg_name
    }
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    # Variable references should not trigger missing-object warnings
    assert "BIGIP6001" not in _codes_set(diags)
    assert "BIGIP6002" not in _codes_set(diags)


# BIGIP6011: Invalid IP address in IP-type data-group records


def test_bigip6011_invalid_ip_in_data_group():
    source = """\
ltm data-group internal /Common/ip_dg {
    records {
        not_an_ip { }
    }
    type ip
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6011" in _codes_set(diags)
    msgs = _msgs(diags, "BIGIP6011")
    assert any("not_an_ip" in m for m in msgs)


def test_bigip6011_valid_ipv4_clean():
    source = """\
ltm data-group internal /Common/ip_dg {
    records {
        10.0.0.1 { }
        192.168.1.0/24 { }
    }
    type ip
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6011" not in _codes_set(diags)


def test_bigip6011_valid_ipv6_clean():
    source = """\
ltm data-group internal /Common/ip_dg {
    records {
        ::1 { }
        2001:db8::/32 { }
    }
    type ip
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6011" not in _codes_set(diags)


def test_bigip6011_string_type_not_checked():
    """Non-IP data-groups should not trigger BIGIP6011."""
    source = """\
ltm data-group internal /Common/str_dg {
    records {
        not_an_ip { }
    }
    type string
}
"""
    config = parse_bigip_conf(source)
    diags = validate_bigip_config(config)
    assert "BIGIP6011" not in _codes_set(diags)
