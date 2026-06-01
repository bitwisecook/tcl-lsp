"""Tests for profile-guided branch-reordering suggestions (P100)."""

from __future__ import annotations

from compiler.pgo import find_pgo_suggestions
from compiler.pgo.profile_data import ProfileData
from compiler.pgo.reorder import REORDER_CODE
from compiler.pgo.trace_parser import parse_f5_log
from shared.text_edits import apply_edits

# An if/elseif equality-dispatch chain (the article's dc_1 idiom, extended
# to three addresses).  Body lines: pool_a=3, pool_b=5, pool_c=7.
DISPATCH = """\
when HTTP_REQUEST {
    if {[IP::remote_addr] eq "10.0.0.1"} {
        pool pool_a
    } elseif {[IP::remote_addr] eq "10.0.0.2"} {
        pool pool_b
    } elseif {[IP::remote_addr] eq "10.0.0.3"} {
        pool pool_c
    }
}
"""


def _apply(source: str, suggestions: list) -> str:
    edits = [
        (s.range.start.offset, s.range.end.offset + 1 - s.range.start.offset, s.replacement)
        for s in suggestions
    ]
    return apply_edits(source, edits)


def test_precise_line_profile_reorders_hottest_first() -> None:
    # pool_b (line 5) is hottest, then pool_c (7), then pool_a (3).
    prof = ProfileData(line_counts={3: 10, 5: 900, 7: 50}, source="test")
    sugs = find_pgo_suggestions(DISPATCH, prof)
    assert len(sugs) == 1
    sug = sugs[0]
    assert sug.code == REORDER_CODE
    assert sug.hint_only is True
    assert "10.0.0.2" in sug.message

    rewritten = _apply(DISPATCH, sugs)
    # Hottest branch tested first, original first branch demoted to last.
    first_if = rewritten.index("if {[IP::remote_addr]")
    assert rewritten[first_if:].startswith('if {[IP::remote_addr] eq "10.0.0.2"}')
    # Order of the three constants in the rewrite reflects the weights.
    order = [rewritten.index(f'"10.0.0.{n}"') for n in (2, 3, 1)]
    assert order == sorted(order)


def test_no_suggestion_when_already_hottest_first() -> None:
    prof = ProfileData(line_counts={3: 900, 5: 50, 7: 10}, source="test")
    assert find_pgo_suggestions(DISPATCH, prof) == []


def test_no_suggestion_without_a_profile() -> None:
    assert find_pgo_suggestions(DISPATCH, ProfileData()) == []


def test_two_branch_if_else_is_not_reordered() -> None:
    # Only one condition to test (plus else) — nothing to reorder.
    src = """\
when HTTP_REQUEST {
    if {[IP::remote_addr] eq "10.0.0.1"} {
        pool a
    } else {
        pool b
    }
}
"""
    prof = ProfileData(line_counts={3: 1, 5: 1000}, source="test")
    assert find_pgo_suggestions(src, prof) == []


def test_else_clause_preserved_last() -> None:
    src = """\
when HTTP_REQUEST {
    if {$x eq "a"} {
        set r 1
    } elseif {$x eq "b"} {
        set r 2
    } elseif {$x eq "c"} {
        set r 3
    } else {
        set r 0
    }
}
"""
    prof = ProfileData(line_counts={3: 1, 5: 1, 7: 999}, source="test")
    sugs = find_pgo_suggestions(src, prof)
    assert len(sugs) == 1
    rewritten = _apply(src, sugs)
    assert rewritten.rstrip().endswith("}")
    # else stays after every elseif.
    assert rewritten.index("else {") > rewritten.rindex("elseif")


# --- safety gate -----------------------------------------------------------


def test_side_effecting_subject_rejected() -> None:
    # HTTP::header replace mutates state — must not be reordered around.
    src = """\
when HTTP_REQUEST {
    if {[HTTP::header replace X] eq "a"} {
        set r 1
    } elseif {[HTTP::header replace X] eq "b"} {
        set r 2
    } elseif {[HTTP::header replace X] eq "c"} {
        set r 3
    }
}
"""
    prof = ProfileData(line_counts={3: 1, 5: 1, 7: 999}, source="test")
    assert find_pgo_suggestions(src, prof) == []


def test_mixed_operators_rejected() -> None:
    src = """\
when HTTP_REQUEST {
    if {$x eq "a"} {
        set r 1
    } elseif {$x ne "b"} {
        set r 2
    } elseif {$x eq "c"} {
        set r 3
    }
}
"""
    prof = ProfileData(line_counts={3: 1, 5: 999, 7: 1}, source="test")
    assert find_pgo_suggestions(src, prof) == []


def test_different_subjects_rejected() -> None:
    src = """\
when HTTP_REQUEST {
    if {$x eq "a"} {
        set r 1
    } elseif {$y eq "b"} {
        set r 2
    } elseif {$x eq "c"} {
        set r 3
    }
}
"""
    prof = ProfileData(line_counts={3: 1, 5: 999, 7: 1}, source="test")
    assert find_pgo_suggestions(src, prof) == []


def test_repeated_constant_rejected() -> None:
    # Two clauses testing the same value are not a clean dispatch.
    src = """\
when HTTP_REQUEST {
    if {$x eq "a"} {
        set r 1
    } elseif {$x eq "a"} {
        set r 2
    } elseif {$x eq "c"} {
        set r 3
    }
}
"""
    prof = ProfileData(line_counts={3: 1, 5: 999, 7: 1}, source="test")
    assert find_pgo_suggestions(src, prof) == []


# --- coarse F5-log attribution ---------------------------------------------


def test_coarse_command_attribution_ranks_distinct_commands() -> None:
    src = (
        "when HTTP_REQUEST {\n"
        '    if {$x eq "a"} { log local0. hit }'
        ' elseif {$x eq "b"} { HTTP::redirect http://x }\n'
        "}\n"
    )
    log = "\n".join(
        [
            "1,RP_CMD_ENTRY,/Common/vs1,HTTP::redirect,1,0x1,a,1,0,b,2,0",
            "2,RP_CMD_ENTRY,/Common/vs1,HTTP::redirect,1,0x1,a,1,0,b,2,0",
            "3,RP_CMD_ENTRY,/Common/vs1,log,1,0x1,a,1,0,b,2,0",
        ]
    )
    sugs = find_pgo_suggestions(src, parse_f5_log(log))
    assert len(sugs) == 1
    assert "b" in sugs[0].message


def test_coarse_cannot_rank_same_command_branches() -> None:
    # pool/pool/pool collide under coarse command attribution — no
    # (potentially wrong) suggestion is emitted.
    log = "\n".join(
        [
            "1,RP_CMD_ENTRY,/Common/vs1,IP::remote_addr,1,0x1,a,1,0,b,2,0",
            "2,RP_EVENT_ENTRY,/Common/vs1,HTTP_REQUEST,1,0x1,a,1,0,b,2,0",
        ]
    )
    assert find_pgo_suggestions(DISPATCH, parse_f5_log(log)) == []


# --- switch arms -----------------------------------------------------------

SWITCH = """\
when HTTP_REQUEST {
    switch [HTTP::path] {
        /a {
            pool pool_a
        }
        /b {
            pool pool_b
        }
        /c {
            pool pool_c
        }
        default {
            pool pool_default
        }
    }
}
"""


def test_switch_reorders_hottest_arm_first_default_last() -> None:
    # Arm bodies: /a line 4, /b line 7, /c line 10.  /b hottest.
    prof = ProfileData(line_counts={4: 5, 7: 500, 10: 30}, source="test")
    sugs = find_pgo_suggestions(SWITCH, prof)
    assert len(sugs) == 1
    assert sugs[0].code == REORDER_CODE
    assert "/b" in sugs[0].message

    rewritten = _apply(SWITCH, sugs)
    # Hottest arm first, default still last, indentation preserved.
    order = [rewritten.index(f"/{c} {{") for c in ("b", "c", "a")]
    assert order == sorted(order)
    assert rewritten.index("default {") > rewritten.index("/a {")
    assert "        /b {\n            pool pool_b" in rewritten


def test_switch_glob_mode_not_reordered() -> None:
    src = """\
when HTTP_REQUEST {
    switch -glob [HTTP::path] {
        /a* { pool a }
        /b* { pool b }
        /c* { pool c }
    }
}
"""
    prof = ProfileData(line_counts={3: 1, 4: 999, 5: 1}, source="t")
    assert find_pgo_suggestions(src, prof) == []


def test_switch_fallthrough_not_reordered() -> None:
    src = """\
when HTTP_REQUEST {
    switch [HTTP::path] {
        /a -
        /b { pool ab }
        /c { pool c }
    }
}
"""
    prof = ProfileData(line_counts={4: 1, 5: 999}, source="t")
    assert find_pgo_suggestions(src, prof) == []


def test_switch_already_hottest_first_not_reordered() -> None:
    prof = ProfileData(line_counts={4: 500, 7: 30, 10: 5}, source="test")
    assert find_pgo_suggestions(SWITCH, prof) == []
