#!/usr/bin/env python3
"""
Dynamic CI Runner for Ferrite Monorepo.

Analyzes changed files via `git diff` against a base commit/branch,
determines which packages and test suites are affected, and executes
the minimal set of `cargo check` and `cargo test` commands.

Usage:
    python3 scripts/ci-affected.py [--base <ref>] [--dry-run]
"""

import argparse
import json
import os
import subprocess
import sys

# Files whose changes trigger a full workspace check and test
GLOBAL_TRIGGER_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
    ".github/",
    "scripts/",
    "crates/contract/",
)

def run_cmd(cmd, cwd=None, capture=True):
    if capture:
        res = subprocess.run(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        return res.returncode, res.stdout.strip(), res.stderr.strip()
    else:
        res = subprocess.run(cmd, cwd=cwd)
        return res.returncode


def get_git_diff_files(base_ref):
    if base_ref:
        for target in [f"{base_ref}...HEAD", base_ref]:
            code, out, _ = run_cmd(["git", "diff", "--name-only", target])
            if code == 0:
                files = [line.strip() for line in out.splitlines() if line.strip()]
                # Also include uncommitted working tree changes
                _, uncommitted, _ = run_cmd(["git", "diff", "--name-only", "HEAD"])
                if uncommitted:
                    files.extend([l.strip() for l in uncommitted.splitlines() if l.strip()])
                _, untracked, _ = run_cmd(["git", "status", "--porcelain"])
                if untracked:
                    for line in untracked.splitlines():
                        if line.startswith("??"):
                            files.append(line[3:].strip())
                files = sorted(set(files))
                return target, files

    # Auto-detect default base branch
    for candidate in ["origin/main", "remotes/newxapi/main", "newxapi/main", "HEAD~1"]:
        code, out, _ = run_cmd(["git", "diff", "--name-only", f"{candidate}...HEAD"])
        if code == 0:
            files = [line.strip() for line in out.splitlines() if line.strip()]
            # Also include working tree changes
            _, uncommitted, _ = run_cmd(["git", "diff", "--name-only", "HEAD"])
            if uncommitted:
                files.extend([l.strip() for l in uncommitted.splitlines() if l.strip()])
            _, untracked, _ = run_cmd(["git", "status", "--porcelain"])
            if untracked:
                for line in untracked.splitlines():
                    if line.startswith("??"):
                        files.append(line[3:].strip())
            files = sorted(set(files))
            return f"{candidate}...HEAD", files

    # Fallback to working tree changes
    _, uncommitted, _ = run_cmd(["git", "diff", "--name-only", "HEAD"])
    files = [l.strip() for l in uncommitted.splitlines() if l.strip()] if uncommitted else []
    return "working-tree", sorted(set(files))


def load_workspace_packages():
    code, out, err = run_cmd(["cargo", "metadata", "--format-version", "1", "--no-deps"])
    if code != 0:
        print(f"Error reading cargo metadata: {err}", file=sys.stderr)
        sys.exit(1)

    data = json.loads(out)
    root = data["workspace_root"]
    packages = []

    for p in data["packages"]:
        manifest_dir = os.path.relpath(os.path.dirname(p["manifest_path"]), root)
        tests_dir = os.path.join(os.path.dirname(p["manifest_path"]), "tests")
        has_tests = os.path.isdir(tests_dir) and any(f.endswith(".rs") for f in os.listdir(tests_dir))
        is_web = manifest_dir.startswith("crates/web") or manifest_dir in ("apps/admin-web", "apps/tavern-web")

        packages.append({
            "name": p["name"],
            "dir": manifest_dir,
            "has_tests": has_tests,
            "is_web": is_web,
        })

    return packages


def determine_affected(changed_files, packages):
    # Check global triggers first
    for f in changed_files:
        for trig in GLOBAL_TRIGGER_PATHS:
            if f == trig or f.startswith(trig):
                return {
                    "is_global": True,
                    "trigger": f,
                    "native_check_pkgs": [p["name"] for p in packages if not p["is_web"]],
                    "wasm_check_pkgs": [p["name"] for p in packages if p["is_web"]],
                    "test_pkgs": [p["name"] for p in packages if p["has_tests"]],
                }

    affected_native_check = set()
    affected_wasm_check = set()
    affected_tests = set()

    for f in changed_files:
        # Match against package directories
        matched = False
        for p in packages:
            p_dir = p["dir"]
            if f == p_dir or f.startswith(p_dir + "/"):
                matched = True
                if p["is_web"]:
                    affected_wasm_check.add(p["name"])
                else:
                    affected_native_check.add(p["name"])

                if p["has_tests"]:
                    affected_tests.add(p["name"])

    return {
        "is_global": False,
        "trigger": None,
        "native_check_pkgs": sorted(affected_native_check),
        "wasm_check_pkgs": sorted(affected_wasm_check),
        "test_pkgs": sorted(affected_tests),
    }


def main():
    parser = argparse.ArgumentParser(description="Run affected tests and checks in Ferrite monorepo")
    parser.add_argument("--base", help="Git base ref or commit to diff against (e.g. origin/main)")
    parser.add_argument("--dry-run", action="store_true", help="Print affected packages without running cargo")
    parser.add_argument("--json", action="store_true", help="Output affected packages as JSON")
    args = parser.parse_args()

    ref_used, changed_files = get_git_diff_files(args.base)
    packages = load_workspace_packages()
    affected = determine_affected(changed_files, packages)

    if args.json:
        print(json.dumps({
            "base_ref": ref_used,
            "changed_files_count": len(changed_files),
            "affected": affected,
        }, indent=2))
        return

    print("=" * 65)
    print("  FERRITE DYNAMIC CI - AFFECTED CRATES RUNNER")
    print("=" * 65)
    print(f"Diff target : {ref_used} ({len(changed_files)} changed files)")

    if affected["is_global"]:
        print(f"Scope       : FULL WORKSPACE (triggered by: {affected['trigger']})")
    else:
        print(f"Scope       : DYNAMIC SELECTIVE")

    print(f"Native check: {len(affected['native_check_pkgs'])} package(s)")
    if affected["native_check_pkgs"]:
        print("              " + ", ".join(affected["native_check_pkgs"]))

    print(f"WASM check  : {len(affected['wasm_check_pkgs'])} package(s)")
    if affected["wasm_check_pkgs"]:
        print("              " + ", ".join(affected["wasm_check_pkgs"]))

    print(f"Cargo test  : {len(affected['test_pkgs'])} package(s)")
    if affected["test_pkgs"]:
        print("              " + ", ".join(affected["test_pkgs"]))
    print("=" * 65)

    if not affected["native_check_pkgs"] and not affected["wasm_check_pkgs"] and not affected["test_pkgs"]:
        print("No crates affected by these changes. Compilation and testing skipped.")
        sys.exit(0)

    if args.dry_run:
        print("[DRY-RUN] Execution skipped.")
        sys.exit(0)

    # 1. Run native checks
    if affected["native_check_pkgs"]:
        print("\n>>> Running cargo check (native)...")
        cmd = ["cargo", "check"]
        for p in affected["native_check_pkgs"]:
            cmd.extend(["-p", p])
        code = run_cmd(cmd, capture=False)
        if code != 0:
            print("ERROR: cargo check (native) failed!", file=sys.stderr)
            sys.exit(code)

    # 2. Run wasm checks
    if affected["wasm_check_pkgs"]:
        print("\n>>> Running cargo check --target wasm32-unknown-unknown...")
        cmd = ["cargo", "check", "--target", "wasm32-unknown-unknown"]
        for p in affected["wasm_check_pkgs"]:
            cmd.extend(["-p", p])
        code = run_cmd(cmd, capture=False)
        if code != 0:
            print("ERROR: cargo check (wasm32) failed!", file=sys.stderr)
            sys.exit(code)

    # 3. Run cargo test on affected packages
    if affected["test_pkgs"]:
        print("\n>>> Running cargo test on affected crates...")
        cmd = ["cargo", "test"]
        for p in affected["test_pkgs"]:
            cmd.extend(["-p", p])
        code = run_cmd(cmd, capture=False)
        if code != 0:
            print("ERROR: cargo test failed!", file=sys.stderr)
            sys.exit(code)

    print("\n" + "=" * 65)
    print("  ALL AFFECTED CHECKS AND TESTS PASSED SUCCESSFULLY! ")
    print("=" * 65)


if __name__ == "__main__":
    main()
