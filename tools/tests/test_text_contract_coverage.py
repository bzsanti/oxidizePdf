"""Adversarial traceability checks for #666; standard library only."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("coverage_validator", ROOT / "tools/text_contracts/validate_coverage.py")
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)
MANIFEST = json.loads((ROOT / "oxidize-pdf-core/tests/fixtures/text_contracts/coverage.json").read_text())


class CoverageTraceability(unittest.TestCase):
    def test_current_manifest_matches_the_agreed_plan(self):
        self.assertEqual(VALIDATOR.validate(MANIFEST, ROOT), [])

    def test_deleted_row_is_detected_against_plan(self):
        data = copy.deepcopy(MANIFEST)
        data["rows"] = data["rows"][1:]
        self.assertTrue(any("missing=['F01']" in error for error in VALIDATOR.validate(data, ROOT)))

    def test_duplicate_row_is_rejected_even_with_all_ids_present(self):
        data = copy.deepcopy(MANIFEST)
        data["rows"].append(data["rows"][0])
        self.assertIn("duplicate coverage ID", VALIDATOR.validate(data, ROOT))

    def test_missing_oracle_or_closure_is_not_traceable(self):
        for key, message in [("oracle_sources", "independent oracle"), ("closure_criterion", "closure_criterion")]:
            data = copy.deepcopy(MANIFEST)
            del data["rows"][0][key]
            self.assertTrue(any(message in error for error in VALIDATOR.validate(data, ROOT)), key)

    def test_invented_test_target_is_rejected(self):
        data = copy.deepcopy(MANIFEST)
        data["rows"][0]["tests"] = ["nonexistent_issue666_contract"]
        self.assertIn("F01: missing test target nonexistent_issue666_contract", VALIDATOR.validate(data, ROOT))

    def test_partial_row_cannot_hide_remaining_work(self):
        data = copy.deepcopy(MANIFEST)
        data["rows"][0]["status"] = "executable-partial"
        data["rows"][0]["remaining"] = ""
        self.assertIn("F01: incomplete row must state remaining work", VALIDATOR.validate(data, ROOT))

    def test_target_path_cannot_escape_the_test_directory(self):
        data = copy.deepcopy(MANIFEST)
        data["rows"][0]["tests"] = ["../../elsewhere"]
        self.assertIn("F01: invalid test target", VALIDATOR.validate(data, ROOT))

    def test_plan_path_cannot_escape_repository(self):
        data = copy.deepcopy(MANIFEST)
        data["plan"] = "../outside-plan.md"
        self.assertEqual(VALIDATOR.validate(data, ROOT), ["plan escapes repository"])


if __name__ == "__main__":
    unittest.main()
