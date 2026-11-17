from pathlib import Path

import ledger_status as ls

HEADER = "crate\ttarget\tpath\tverdict\tcause\tevidence\tsurvivor"


def write(path: Path, header: str, rows: list[str]) -> None:
    path.write_text("\n".join([header, *rows]) + "\n", encoding="utf-8")


def test_a_ledger_without_the_column_is_all_open(tmp_path):
    write(tmp_path / "a.tsv", HEADER, ["c\tlib\tx::y\tKEEP-ORACLE\tslug\tev\t", "c\tlib\tx::z\tRETIRE\ts\te\t"])
    counts, unknown = ls.read_statuses(tmp_path / "a.tsv")
    assert counts == {"open": 2} and unknown == []


def test_statuses_are_counted_and_strangers_reported(tmp_path):
    write(
        tmp_path / "b.tsv",
        HEADER + "\tstatus\tnote",
        [
            "c\tlib\ta\tFIX-FIXTURE\ts\te\t\trepaired\tcommit abc",
            "c\tlib\tb\tFIX-FIXTURE\ts\te\t\texposed\tfails on the product",
            "c\tlib\tc\tKEEP-ORACLE\ts\te\t\t\t",
            "c\tlib\td\tKEEP-ORACLE\ts\tsays \"quoted\"\t\tfixture repaired abc; passes\t",
            "",
        ],
    )
    counts, unknown = ls.read_statuses(tmp_path / "b.tsv")
    assert counts == {"repaired": 1, "exposed": 1, "open": 2}
    assert unknown == [(5, "fixture repaired abc; passes")]


def test_table_skips_side_files_and_check_fails_on_a_stranger(tmp_path, capsys):
    write(tmp_path / "area.tsv", HEADER + "\tstatus", ["c\tlib\ta\tMERGE\ts\te\tsurv\tdeleted"])
    write(tmp_path / "area-clusters.tsv", "cluster\tsize", ["x\t2"])
    text, bad = ls.table(tmp_path)
    assert bad == 0 and "| area | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 1 |" in text and "clusters" not in text
    write(tmp_path / "other.tsv", HEADER + "\tstatus", ["c\tlib\ta\tMERGE\ts\te\tsurv\tdone"])
    assert ls.main([str(tmp_path), "--check"]) == 1
    assert "other.tsv:2" in capsys.readouterr().err
    assert ls.main([str(tmp_path)]) == 0
