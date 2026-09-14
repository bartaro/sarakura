import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "tools" / "rights_name_guard.py"


class RightsNameGuardTests(unittest.TestCase):
    # Use a synthetic title to check web-document suffix coverage and ensure the
    # matched text itself is not printed in the scanner output.
    def test_detects_protected_term_in_web_manual(self):
        for suffix in (".html", ".htm", ".js", ".css", ".svg"):
            with self.subTest(suffix=suffix), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                denylist = root / "deny_terms.local.txt"
                denylist.write_text("PROTECTED_TITLE_ALPHA\n", encoding="utf-8")
                (root / ("manual" + suffix)).write_text("PROTECTED_TITLE_ALPHA", encoding="utf-8")
                result = self.run_guard(root, denylist)
                self.assertEqual(result.returncode, 1)
                self.assertNotIn("PROTECTED_TITLE_ALPHA", result.stdout)

    # Launch the scanner with the same Python interpreter and capture both streams;
    # return its status for assertions rather than raising on an expected match.
    def run_guard(self, root: Path, denylist: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root), "--denylist", str(denylist)],
            text=True,
            capture_output=True,
            check=False,
        )

    # Check that a synthetic denied name in Rust source produces exit status 1.
    def test_detects_protected_term_in_text(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            denylist = root / "deny_terms.local.txt"
            denylist.write_text("PROTECTED_TITLE_ALPHA\n", encoding="utf-8")
            (root / "source.rs").write_text("PROTECTED_TITLE_ALPHA\n", encoding="utf-8")
            self.assertEqual(self.run_guard(root, denylist).returncode, 1)

    # Check that a clean source passes even when a generated target file contains
    # the synthetic term. The private denylist itself must also be excluded.
    def test_accepts_clean_text_and_skips_generated_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            denylist = root / "deny_terms.local.txt"
            denylist.write_text("PROTECTED_TITLE_ALPHA\n", encoding="utf-8")
            (root / "source.rs").write_text("generic rule fixture\n", encoding="utf-8")
            generated = root / "target"
            generated.mkdir()
            (generated / "generated.txt").write_text("PROTECTED_TITLE_ALPHA\n", encoding="utf-8")
            self.assertEqual(self.run_guard(root, denylist).returncode, 0)


if __name__ == "__main__":
    unittest.main()
