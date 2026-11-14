#!/usr/bin/env python3
"""
Attune E2E Test Suite Runner — Windows Runtime, Packaging, Code Signing & Verification Pipeline.

Usage:
    python tests/e2e_windows_pipeline/run_e2e_tests.py --tier 1
    python tests/e2e_windows_pipeline/run_e2e_tests.py --all
    python tests/e2e_windows_pipeline/run_e2e_tests.py --tier 1 --json-report test_report.json
    python tests/e2e_windows_pipeline/run_e2e_tests.py --test-file tests/e2e_windows_pipeline/test_tier1_feature_coverage.py

Exit Code:
    0: All requested tests passed.
    1: One or more tests failed or encountered errors.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import unittest
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

TEST_DIR = Path(__file__).resolve().parent
REPO_ROOT = TEST_DIR.parent.parent


class E2ETestResult(unittest.TextTestResult):
    """Custom TestResult collecting detailed per-test execution data for JSON reporting."""

    def __init__(self, stream, descriptions, verbosity):
        super().__init__(stream, descriptions, verbosity)
        self.records: List[Dict[str, Any]] = []
        self._test_start_time: float = 0.0

    def startTest(self, test: unittest.TestCase) -> None:
        self._test_start_time = time.perf_counter()
        super().startTest(test)

    def addSuccess(self, test: unittest.TestCase) -> None:
        duration = time.perf_counter() - self._test_start_time
        self.records.append({
            "test": str(test),
            "name": test._testMethodName,
            "class": test.__class__.__name__,
            "module": test.__class__.__module__,
            "status": "PASS",
            "duration_seconds": round(duration, 4),
            "message": "",
        })
        super().addSuccess(test)

    def addFailure(self, test: unittest.TestCase, err) -> None:
        duration = time.perf_counter() - self._test_start_time
        msg = self._exc_info_to_string(err, test)
        self.records.append({
            "test": str(test),
            "name": test._testMethodName,
            "class": test.__class__.__name__,
            "module": test.__class__.__module__,
            "status": "FAIL",
            "duration_seconds": round(duration, 4),
            "message": msg,
        })
        super().addFailure(test, err)

    def addError(self, test: unittest.TestCase, err) -> None:
        duration = time.perf_counter() - self._test_start_time
        msg = self._exc_info_to_string(err, test)
        self.records.append({
            "test": str(test),
            "name": test._testMethodName,
            "class": test.__class__.__name__,
            "module": test.__class__.__module__,
            "status": "ERROR",
            "duration_seconds": round(duration, 4),
            "message": msg,
        })
        super().addError(test, err)

    def addSkip(self, test: unittest.TestCase, reason: str) -> None:
        duration = time.perf_counter() - self._test_start_time
        self.records.append({
            "test": str(test),
            "name": test._testMethodName,
            "class": test.__class__.__name__,
            "module": test.__class__.__module__,
            "status": "SKIP",
            "duration_seconds": round(duration, 4),
            "message": reason,
        })
        super().addSkip(test, reason)


class E2ETestRunner(unittest.TextTestRunner):
    resultclass = E2ETestResult

    def run(self, test: unittest.TestSuite) -> E2ETestResult:
        return super().run(test)  # type: ignore


def discover_tier_tests(tier: str) -> unittest.TestSuite:
    """Discovers tests belonging to a specific tier or all tiers."""
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    tier_map = {
        "1": "test_tier1_*.py",
        "2": "test_tier2_*.py",
        "3": "test_tier3_*.py",
        "4": "test_tier4_*.py",
    }

    if tier.lower() in ("all", "0"):
        pattern = "test_*.py"
    else:
        pattern = tier_map.get(tier, f"test_tier{tier}_*.py")

    discovered = loader.discover(start_dir=str(TEST_DIR), pattern=pattern)
    suite.addTests(discovered)
    return suite


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Attune E2E Windows Runtime, Packaging & Verification Test Runner"
    )
    parser.add_argument(
        "--tier",
        type=str,
        default="1",
        help="Test tier to execute: '1' (Feature Coverage), '2' (Boundary/Corner), '3' (Pairwise), '4' (Real-World), or 'all'",
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="Execute all tiers across the entire E2E test suite",
    )
    parser.add_argument(
        "--test-file",
        type=str,
        default="",
        help="Execute a specific test file path directly",
    )
    parser.add_argument(
        "--json-report",
        type=str,
        default="",
        help="Path to output JSON test execution report",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        default=True,
        help="Enable verbose test reporting",
    )

    args = parser.parse_args()

    # Ensure repository root is on sys.path
    if str(REPO_ROOT) not in sys.path:
        sys.path.insert(0, str(REPO_ROOT))

    loader = unittest.TestLoader()

    if args.test_file:
        test_path = Path(args.test_file)
        if not test_path.is_absolute():
            test_path = REPO_ROOT / test_path
        if not test_path.exists():
            print(f"Error: Specified test file does not exist: {test_path}", file=sys.stderr)
            return 1
        suite = loader.discover(start_dir=str(test_path.parent), pattern=test_path.name)
        tier_label = f"file:{test_path.name}"
    elif args.all:
        suite = discover_tier_tests("all")
        tier_label = "all"
    else:
        suite = discover_tier_tests(args.tier)
        tier_label = f"Tier {args.tier}"

    total_tests = suite.countTestCases()
    print("=" * 78)
    print(f" Attune Windows Pipeline E2E Test Suite — {tier_label}")
    print(f" Total Discovered Test Cases: {total_tests}")
    print(f" Timestamp: {datetime.now(timezone.utc).isoformat()}")
    print("=" * 78)

    runner = E2ETestRunner(verbosity=2 if args.verbose else 1)
    start_time = time.perf_counter()
    result = runner.run(suite)
    elapsed_time = time.perf_counter() - start_time

    pass_count = len(result.records) - len(result.failures) - len(result.errors) - len(result.skipped)
    fail_count = len(result.failures)
    error_count = len(result.errors)
    skip_count = len(result.skipped)

    print("\n" + "=" * 78)
    print(" Execution Summary:")
    print(f"   Total:    {len(result.records)}")
    print(f"   Passed:   {pass_count}")
    print(f"   Failed:   {fail_count}")
    print(f"   Errors:   {error_count}")
    print(f"   Skipped:  {skip_count}")
    print(f"   Duration: {elapsed_time:.3f} seconds")
    print("=" * 78)

    # Export JSON report if requested
    if args.json_report:
        report_data = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "tier": tier_label,
            "duration_seconds": round(elapsed_time, 4),
            "summary": {
                "total": len(result.records),
                "passed": pass_count,
                "failed": fail_count,
                "errors": error_count,
                "skipped": skip_count,
            },
            "results": result.records,
        }
        report_path = Path(args.json_report)
        if not report_path.is_absolute():
            report_path = REPO_ROOT / report_path
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report_data, indent=2), encoding="utf-8")
        print(f" JSON test report written to: {report_path}")

    # Return 0 if all tests passed and no errors, 1 otherwise
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
