#!/usr/bin/env python3
"""Lists the Rust crates linked into patina-compositor and patina-panel, with their licences.

Run from the repository root; prints to stdout. Needs `cargo` and a Cargo.lock.
"""
import json
import subprocess
import sys

BINARIES = ("patina-compositor", "patina-panel")


def main() -> int:
    host = next(
        line.split(": ", 1)[1]
        for line in subprocess.check_output(["rustc", "-vV"], text=True).splitlines()
        if line.startswith("host: ")
    )
    meta = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", host],
            text=True,
        )
    )
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    roots = [pid for pid, p in packages.items() if p["name"] in BINARIES]

    seen, stack = set(), list(roots)
    while stack:
        pid = stack.pop()
        if pid in seen:
            continue
        seen.add(pid)
        # Normal dependencies only: dev- and build-only crates are not linked into the binaries.
        for dep in nodes[pid]["deps"]:
            if any(k["kind"] in (None, "normal") for k in dep["dep_kinds"]):
                stack.append(dep["pkg"])

    rows = sorted(
        (packages[p]["name"], packages[p]["version"], packages[p].get("license") or "see crate")
        for p in seen
        if packages[p]["source"] is not None  # skip our own workspace crates
    )
    print("Rust crates statically linked into patina-compositor and patina-panel.")
    print("Each crate is distributed under its own licence; the full texts are in the crate sources")
    print("(https://crates.io/crates/<name>/<version>).")
    print()
    for name, version, lic in rows:
        print(f"{name} {version}\t{lic}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
