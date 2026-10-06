"""Do not let feature-gated empty suites or ignored contracts produce green CI."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("fast", Path(__file__).parents[1] / "text_contracts/run_fast.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ExecutedContracts(unittest.TestCase):
    def test_complete_nonempty_targets(self):
        module.verify_contract_log("test result: ok. 3 passed; 0 failed; 0 ignored;\n"
                                   "test result: ok. 2 passed; 0 failed; 0 ignored;\n", 2)

    def test_zero_tests_are_not_feature_coverage(self):
        with self.assertRaises(ValueError):
            module.verify_contract_log("test result: ok. 0 passed; 0 failed; 0 ignored;\n", 1)

    def test_ignored_contracts_fail_the_gate(self):
        with self.assertRaises(ValueError):
            module.verify_contract_log("test result: ok. 2 passed; 0 failed; 1 ignored;\n", 1)

    def test_missing_or_duplicate_target_summary_fails(self):
        line = "test result: ok. 3 passed; 0 failed; 0 ignored;\n"
        for text, expected in [(line, 2), (line * 2, 1), ("", 1)]:
            with self.subTest(text=text, expected=expected), self.assertRaises(ValueError):
                module.verify_contract_log(text, expected)

    def test_failed_summary_cannot_count_as_success(self):
        with self.assertRaises(ValueError):
            module.verify_contract_log("test result: FAILED. 2 passed; 1 failed; 0 ignored;\n", 1)
