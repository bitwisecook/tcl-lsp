from pathlib import Path

import pytest

import scrub_inventory as inv


def test_scan_blanks_strings_and_comments_but_keeps_length():
    src = 'let a = "{ not a brace }"; // } also not\nlet b = r#"x " y"#; /* { /* nested */ } */ {}'
    scan = inv.scan_rust(src)
    assert len(scan.skeleton) == len(src)
    assert scan.skeleton.count("{") == 1 and scan.skeleton.count("}") == 1
    assert [s[2] for s in scan.strings] == ["{ not a brace }", 'x " y']
    assert len(scan.comments) == 2


def test_scan_tells_a_char_literal_from_a_lifetime():
    src = "fn f<'a>(x: &'a str) -> char { let c = '{'; let d = '\\''; c }"
    scan = inv.scan_rust(src)
    assert scan.skeleton.count("{") == 1 and scan.skeleton.count("}") == 1
    assert "'a" in scan.skeleton


def test_unescape_handles_continuations_and_escapes():
    assert inv._unescape("a\\n\\\n   b\\\"c\\u{41}") == 'a\nb"cA'


@pytest.mark.parametrize(
    ("literal", "expected"),
    [
        ("set x 1\nputs $x", True),
        ("proc f {} { return 1 }", True),
        ("expected {src} to fire, got {codes:?}", False),
        ("W210", False),
        ("a dead arm reports nothing", False),
        ("if {$x} {{ puts {a} }}", True),
    ],
)
def test_is_fixture(literal, expected):
    assert inv.is_fixture(literal) is expected


def _crate(tmp_path: Path, name: str = "demo") -> Path:
    crate = tmp_path / "rust" / name
    (crate / "src").mkdir(parents=True)
    (crate / "tests" / "it_2").mkdir(parents=True)
    (crate / "Cargo.toml").write_text(f'[package]\nname = "{name}"\n[dependencies]\nname = "x"\n')
    return crate


def test_tests_in_file_reads_modules_fixtures_and_citations(tmp_path):
    crate = _crate(tmp_path)
    file = crate / "src" / "walk.rs"
    file.write_text(
        """
mod tests {
    use super::*;

    // tclsh 8.6.18 prints 2.
    #[test]
    fn folds_a_constant() {
        let src = "set x 1\\nputs [expr {$x + 1}]";
        assert!(fires(src, "O100"), "expected {src}");
    }

    mod inner {
        #[test]
        fn holds_a_structure() {
            assert_eq!(Card::new().len(), 0);
        }
    }
}
"""
    )
    tests = inv.tests_in_file(tmp_path, file)
    assert [t.path for t in tests] == ["walk::tests::folds_a_constant", "walk::tests::inner::holds_a_structure"]
    first, second = tests
    assert (first.crate, first.target) == ("demo", "lib")
    assert first.fixtures == ("set x 1 puts [expr {$x + 1}]",)
    assert first.codes == ("O100",)
    assert first.oracle and first.value_class == "A-oracle"
    assert "fires" in first.helpers
    assert second.fixtures == () and second.value_class == "D-internal"


def test_integration_target_and_failing_match_by_longest_suffix(tmp_path):
    crate = _crate(tmp_path)
    a = crate / "tests" / "it_2" / "alpha.rs"
    b = crate / "tests" / "it_2" / "beta.rs"
    body = '#[test]\nfn same_name() { check("puts $x"); }\n'
    a.write_text(body)
    b.write_text(body)
    tests = inv.collect(tmp_path, ["rust"])
    assert {(t.target, t.module) for t in tests} == {("it_2", "alpha"), ("it_2", "beta")}
    lst = tmp_path / "failing.txt"
    lst.write_text("# header\ndemo::it_2 beta::same_name\nother nothing::here\n")
    failing = inv.read_failing([lst])
    assert inv.mark_failing(tests, failing) == 1
    assert [t.module for t in tests if t.failing] == ["beta"]


def test_outputs_report_exact_and_overlap_groups(tmp_path):
    crate = _crate(tmp_path)
    (crate / "tests" / "it_2" / "alpha.rs").write_text(
        '#[test]\nfn one() { assert!(fires("set x 1; puts $x", "W210")); }\n'
        '#[test]\nfn two() { assert!(fires("set x 1; puts $x", "W210")); }\n'
    )
    (crate / "src" / "lib.rs").write_text(
        '#[cfg(test)]\nmod tests {\n    #[test]\n    fn three() { let s = "set x 1; puts $x"; assert!(w210(s), "W210"); }\n}\n'
    )
    tests = inv.collect(tmp_path, ["rust"])
    out = tmp_path / "out"
    inv.write_outputs(tests, out, 0)
    exact = (out / "clusters_exact.tsv").read_text().splitlines()
    overlap = (out / "clusters_overlap.tsv").read_text().splitlines()
    assert len(exact) == 3
    assert len(overlap) == 3
    shared = (out / "fixtures_shared.tsv").read_text().splitlines()
    assert shared[1].startswith("3\t2\t1\t0\t")
    assert "Tests read: 3" in (out / "summary.md").read_text()
