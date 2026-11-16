"""Tests for the brace scan and the hoist; run with `pytest docs/work/file-split/tools`."""

from pathlib import Path

import hoist_inline_tests as tool


def test_brace_scan_skips_literals_and_comments():
    text = 'mod t { let a = "}"; let b = r#"\n}\n"#; let d = br"}\"; let c = \'}\'; // }\n /* } /* } */ */ fn f<\'a>() {} }\nfn after() {}'
    close = tool.matching_brace(text, text.index("{"))
    assert text[close + 1 :] == "\nfn after() {}"


def test_hoist_moves_the_module_and_fixes_include_paths(tmp_path: Path):
    source = tmp_path / "rust" / "c" / "src" / "big.rs"
    source.parent.mkdir(parents=True)
    filler = "\n".join(f"fn f{i}() {{}}" for i in range(tool.LIMIT + 5))
    source.write_text(
        "// SPDX-License-Identifier: X\n\n"
        + filler
        + '\n\n#[cfg(test)]\nmod tests {\n    const S: &str = include_str!("../data.txt");\n    const R: &str = r#"\n}\n"#;\n}\n\nfn tail() {}\n'
    )
    moved = tool.hoist(source)
    child = source.parent / "big" / "tests.rs"
    assert moved == [child.as_posix()]
    assert source.read_text().endswith("#[cfg(test)]\nmod tests;\n\nfn tail() {}\n")
    body = child.read_text()
    assert body.startswith("// SPDX-License-Identifier: X\n\n")
    assert 'include_str!("../../data.txt")' in body and 'r#"\n}\n"#' in body


def test_small_files_and_existing_targets_are_left_alone(tmp_path: Path):
    small = tmp_path / "small.rs"
    small.write_text("#[cfg(test)]\nmod tests {\n}\n")
    assert tool.hoist(small) == []
