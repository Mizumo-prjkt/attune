"""
Attune E2E Test Suite — Windows Runtime, Packaging, Code Signing & Verification Pipeline.

This package provides the opaque-box and requirement-driven end-to-end testing harness,
fixtures, and test suites across all tiers (Tiers 1–4) as specified in:
- ORIGINAL_REQUEST.md
- PROJECT.md
- TEST_INFRA.md
- SCOPE.md
"""

from .test_harness import (
    REPO_ROOT,
    TOOLS_DIR,
    PACKAGING_DIR,
    WORKFLOWS_DIR,
    CRATES_DIR,
    REQUIRED_DYNAMIC_DLLS,
    REQUIRED_PE_BINARIES,
    InnoSetupParser,
    WorkflowValidator,
    PEParser,
    AuthenticodeHelper,
    WindowsStagingFixture,
    RustLoaderContract,
    Base64CertHelper,
)

__all__ = [
    "REPO_ROOT",
    "TOOLS_DIR",
    "PACKAGING_DIR",
    "WORKFLOWS_DIR",
    "CRATES_DIR",
    "REQUIRED_DYNAMIC_DLLS",
    "REQUIRED_PE_BINARIES",
    "InnoSetupParser",
    "WorkflowValidator",
    "PEParser",
    "AuthenticodeHelper",
    "WindowsStagingFixture",
    "RustLoaderContract",
    "Base64CertHelper",
]
