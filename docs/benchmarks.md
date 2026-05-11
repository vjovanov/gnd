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
| Timestamp | 2026-05-11T22:23:04+02:00 |
| Host | kung-fu-laptop |
| Os | Linux-6.17.0-14-generic-x86_64-with-glibc2.39 |
| Kernel | Linux kung-fu-laptop 6.17.0-14-generic #14~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC Thu Jan 15 15:52:10 UTC 2 x86_64 x86_64 |
| Cpu | 12th Gen Intel(R) Core(TM) i7-12800H |
| Logical Cpus | 20 |
| Memory | 62.5 GiB |
| Repo Disk Free | 11.9 GiB |
| Python | 3.12.12 |

## Tool Versions

| Tool | Version |
|---|---|
| `grund` | `grund 0.1.0` |
| `lychee` | `lychee 0.23.0` |
| Git commit | `6865906555c644d1e446af7435dd94d5219ea2c4` |
| Git branch | `main` |
| Working tree dirty | `yes` |

## Workload

| Tool | Local work checked |
|---|---:|
| `grund check .` | 201 declarations + 1,553 citations across 48 scanned files (9,715 lines) |
| `lychee --include-fragments README.md docs examples` | 772 links across 56 markup files |

## Results

| Command | Cold | Warm median | Warm min | Warm max |
|---|---:|---:|---:|---:|
| `target/release/grund check .` | 0.023s | 0.025s | 0.020s | 0.029s |
| `target/release/grund fmt --check .` | 0.012s | 0.012s | 0.010s | 0.013s |
| `lychee --no-progress --include-fragments README.md docs examples` | 0.529s | 0.522s | 0.271s | 0.538s |

## Throughput

At the warm median, `grund check .` scans about 392k lines of source per second — roughly the throughput figure on the README badge.

## Comparison

`grund check .` checks 2.3x as many local intent edges as Lychee checks links in this run.
Using warm medians, `grund check .` is 21.1x faster than the configured Lychee run.
Per checked edge, `grund` costs about 14 microseconds; Lychee costs about 676 microseconds per link.

Product copy:

> Lychee checks whether Markdown links still open. `grund` checks whether the project still knows why the code exists.
> On this local run, `grund` checks more than twice as many project-intent edges and finishes several times faster.

## Raw Warm Samples

- `grund check`: 0.025s, 0.029s, 0.027s, 0.021s, 0.027s, 0.023s, 0.020s
- `grund fmt --check`: 0.012s, 0.012s, 0.010s, 0.011s, 0.013s, 0.011s, 0.012s
- `lychee`: 0.522s, 0.526s, 0.271s, 0.504s, 0.538s, 0.507s, 0.526s
