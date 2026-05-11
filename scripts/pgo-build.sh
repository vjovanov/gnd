#!/usr/bin/env bash
set -euo pipefail

# Profile-guided-optimization build of the `grund` release binary (§DA-pgo-release).
# This is a release/benchmarking tool, not part of the normal development build
# or push/PR CI loop.
#
# Three phases: build an instrumented binary (`-Cprofile-generate`), run the
# §AS-benchmarks workload against this repo's own conformant tree to produce
# `.profraw` profiles, merge them, then rebuild the release binary with
# `-Cprofile-use`. The training workload is deliberately the same set of
# commands `benches/instructions.rs` benchmarks — the ones agents and CI invoke
# most — so the profile reflects the hot paths the tool actually runs.
#
# Output: target/release/grund, optimized against the merged profile.
# Requires: the `llvm-tools-preview` rustup component (`llvm-profdata`).
#
# `cargo install grund` from source does not run this — a plain `cargo build
# --release` has no profile to use. The release pipeline (§RM-distribution) runs
# this script to produce the distributed binary; benchmarking can also run it
# when comparing the optimized release artifact. The napi-rs / PyO3 builds get
# the same treatment when they land.

cd "$(dirname "$0")/.."
repo="$PWD"
pgo_dir="$repo/target/pgo-data"
profdata="$pgo_dir/merged.profdata"

# llvm-profdata ships in the active toolchain's llvm-tools-preview component.
llvm_profdata="$(find "$(rustc --print sysroot)" -type f -name 'llvm-profdata*' | head -n1)"
if [ -z "$llvm_profdata" ]; then
  echo "error: llvm-profdata not found — run: rustup component add llvm-tools-preview" >&2
  exit 1
fi

rm -rf "$pgo_dir"
mkdir -p "$pgo_dir"

echo "==> 1/3  build instrumented binary (-Cprofile-generate)"
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Cprofile-generate=$pgo_dir" cargo build --release --locked

grund="$repo/target/release/grund"

echo "==> 2/3  training run — the §AS-benchmarks workload"
# Keep this list in sync with benches/instructions.rs. Exit codes are irrelevant
# here (a non-canonical tree makes `fmt --check` exit 1); we only want the code
# paths exercised.
set +e
for _ in 1 2 3; do
  "$grund" check "$repo"                  >/dev/null 2>&1
  "$grund" list "$repo"                   >/dev/null 2>&1
  "$grund" show FS-check "$repo"          >/dev/null 2>&1
  "$grund" refs G-fast-feedback "$repo"   >/dev/null 2>&1
  "$grund" cover "$repo"                  >/dev/null 2>&1
  "$grund" fmt --check "$repo"            >/dev/null 2>&1
done
set -e

"$llvm_profdata" merge -o "$profdata" "$pgo_dir"/*.profraw

echo "==> 3/3  rebuild optimized (-Cprofile-use)"
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Cprofile-use=$profdata -Cllvm-args=-pgo-warn-missing-function" cargo build --release --locked

echo "==> done: $grund"
"$grund" --version
