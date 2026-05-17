# Local benchmark report

This report is a local wall-clock snapshot for the `grund` repo. It complements the instruction-counting CI benchmark in [§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) and the baseline work tracked by [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets). It is meant for product-facing comparisons with Lychee; it is not the release-blocking regression meter.

## Instructions

Regenerate this report from the repository root:

```sh
python3 scripts/local-benchmark-report.py --out docs/benchmarks.md
```

Useful options:

- `--warm-runs N` changes the number of warm samples after the cold run.
- `--grund PATH` points at a specific `grund` binary; by default the script uses `target/release/grund` when present.
- `--lychee PATH` points at a specific Lychee binary.
- `--lychee-path PATH` may be repeated to replace the default Lychee inputs: `README.md docs examples`.

For a fair local comparison, build the release binary first:

```sh
cargo build --release --locked
python3 scripts/local-benchmark-report.py --out docs/benchmarks.md
```

## Method

- Cold time is the first measured invocation of each exact command in this script run. The script does not use `sudo` and does not drop the OS page cache.
- Warm time is the median of 7 immediate subsequent invocations with command output suppressed.
- Timings use Python `time.perf_counter()` around the whole subprocess, so process startup and argument parsing are included.
- Lychee may perform URL work and may benefit from its own cache or network conditions; `grund` works only over the local scanned tree.

## Machine

| Field | Value |
|---|---|
| Timestamp | 2026-05-17T17:20:27+02:00 |
| Host | kung-fu-workstation |
| Os | Linux-6.17.0-29-generic-x86_64-with-glibc2.39 |
| Kernel | Linux kung-fu-workstation 6.17.0-29-generic #29~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC Mon May 11 10:30:58 UTC 2 x86_64 x86_64 |
| Cpu | Intel(R) Core(TM) Ultra 9 285K |
| Logical Cpus | 24 |
| Memory | 93.7 GiB |
| Repo Disk Free | 23.7 GiB |
| Python | 3.11.5 |

## Tool Versions

| Tool | Version |
|---|---|
| `grund` | `grund 0.1.0` |
| `lychee` | `lychee 0.23.0` |
| Git commit | `f9ab038e2ca9030c0b94d53e83901e81883d9e80` |
| Git branch | `init-workspace-members-section` |
| Working tree dirty | `no` |

## Workload

| Tool | Local work checked |
|---|---:|
| `grund check .` | 313 declarations + 2,006 citations across 85 scanned files (15,709 lines) |
| `lychee --include-fragments README.md docs examples` | 985 links across 74 markup files |

## Results

| Command | Cold | Warm median | Warm min | Warm max |
|---|---:|---:|---:|---:|
| `target/release/grund check .` | 0.029s | 0.027s | 0.027s | 0.028s |
| `target/release/grund fmt --check .` | 0.038s | 0.036s | 0.033s | 0.038s |
| `lychee --no-progress --include-fragments README.md docs examples` | 0.790s | 0.483s | 0.242s | 0.543s |

## Throughput

At the warm median, `grund check .` scans about 574k lines of source per second — roughly the throughput figure on the README badge.

## Comparison

`grund check .` checks 2.4x as many local intent edges as Lychee checks links in this run.
Using warm medians, `grund check .` is 17.7x faster than the configured Lychee run.
Per checked edge, `grund` costs about 12 microseconds; Lychee costs about 490 microseconds per link.

Product copy:

> Lychee checks whether Markdown links still open. `grund` checks whether the project still knows why the code exists.
> On this local run, `grund` checks more than twice as many project-intent edges and finishes several times faster.

## Raw Warm Samples

- `grund check`: 0.028s, 0.027s, 0.027s, 0.027s, 0.027s, 0.027s, 0.027s
- `grund fmt --check`: 0.037s, 0.036s, 0.033s, 0.038s, 0.038s, 0.035s, 0.034s
- `lychee`: 0.482s, 0.543s, 0.248s, 0.506s, 0.495s, 0.483s, 0.242s
