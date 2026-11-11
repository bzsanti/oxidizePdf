import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("ci_install_apt.sh")


class AptInstallTests(unittest.TestCase):
    def run_install(self, failure=""):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = root / "calls.jsonl"
            stub = '''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
name = Path(sys.argv[0]).name
with open(os.environ["APT_TEST_LOG"], "a") as out:
    out.write(json.dumps([name, *sys.argv[1:]]) + "\\n")
if name == "sudo" and sys.argv[1] == "tee":
    sys.stdin.read()
failure = os.environ["APT_TEST_FAILURE"]
if (failure == "preflight" and name == "curl") or (failure == "update" and sys.argv[1:3] == ["apt-get", "update"]):
    sys.exit(42)
'''
            for command in ("curl", "sudo"):
                path = root / command
                path.write_text(stub)
                path.chmod(0o755)
            env = dict(os.environ, PATH=f"{root}:{os.environ['PATH']}",
                       APT_TEST_LOG=str(log), APT_TEST_FAILURE=failure)
            result = subprocess.run(
                ["bash", str(SCRIPT), "--no-install-recommends", "qpdf", "poppler-utils"],
                env=env, capture_output=True, text=True,
            )
            calls = [json.loads(line) for line in log.read_text().splitlines()]
            return result.returncode, calls

    def test_success_preserves_install_arguments(self):
        status, calls = self.run_install()
        self.assertEqual(status, 0)
        self.assertEqual(len(calls), 4)
        self.assertEqual(calls[1], ["sudo", "tee", "/etc/apt/apt-mirrors.txt"])
        self.assertEqual(calls[2][:3], ["sudo", "apt-get", "update"])
        self.assertIn("APT::Update::Error-Mode=any", calls[2])
        self.assertEqual(calls[3][:4], ["sudo", "apt-get", "install", "-y"])
        self.assertEqual(calls[3][-3:], ["--no-install-recommends", "qpdf", "poppler-utils"])

    def test_preflight_failure_never_changes_system(self):
        status, calls = self.run_install("preflight")
        self.assertEqual(status, 42)
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][0], "curl")

    def test_update_failure_never_installs(self):
        status, calls = self.run_install("update")
        self.assertEqual(status, 42)
        self.assertEqual(len(calls), 3)
        self.assertEqual(calls[-1][:3], ["sudo", "apt-get", "update"])
