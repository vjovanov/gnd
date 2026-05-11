# Local benchmark report

This report is a local wall-clock snapshot for the `gnd` repo. It complements the instruction-counting CI benchmark in [§AS-benchmarks](architectural-spec/AS-benchmarks.md#as-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) and the baseline work tracked by [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-g-fast-feedback-budgets). It is meant for product-facing comparisons with Lychee; it is not the release-blocking regression meter.

## Instructions

Regenerate this report from the repository root:

```sh
python3 scripts/local-benchmark-report.py --out docs/benchmarks.md
```

Useful options:

- `--warm-runs N` changes the number of warm samples after the cold run.
- `--gnd PATH` points at a specific `gnd` binary; by default the script uses `target/release/gnd` when present.
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
- Lychee may perform URL work and may benefit from its own cache or network conditions; `gnd` works only over the local scanned tree.

## Machine

| Field | Value |
|---|---|
| Timestamp | 2026-05-11T17:18:18+02:00 |
| Host | kung-fu-laptop |
| Os | Linux-6.17.0-14-generic-x86_64-with-glibc2.39 |
| Kernel | Linux kung-fu-laptop 6.17.0-14-generic #14~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC Thu Jan 15 15:52:10 UTC 2 x86_64 x86_64 |
| Cpu | 12th Gen Intel(R) Core(TM) i7-12800H |
| Logical Cpus | 20 |
| Memory | 62.5 GiB |
| Repo Disk Free | 13.0 GiB |
| Python | 3.12.12 |

## Tool Versions

| Tool | Version |
|---|---|
| `gnd` | `gnd 0.1.0` |
| `lychee` | `lychee 0.23.0` |
| Git commit | `7f577a1cb3a1ee11986eebd01c32d81c1f50dec4` |
| Git branch | `main` |
| Working tree dirty | `yes` |

## Workload

| Tool | Local work checked |
|---|---:|
| `gnd check .` | 198 declarations + 1,505 citations across 47 scanned files |
| `lychee --include-fragments README.md docs examples` | 745 links across 55 markup files |

## Results

| Command | Cold | Warm median | Warm min | Warm max |
|---|---:|---:|---:|---:|
| `target/release/gnd check .` | 0.030s | 0.027s | 0.027s | 0.035s |
| `target/release/gnd fmt --check .` | 0.016s | 0.019s | 0.017s | 0.019s |
| `lychee --no-progress --include-fragments README.md docs examples` | 0.148s | 0.162s | 0.155s | 0.193s |

## Comparison

`gnd check .` checks 2.3x as many local intent edges as Lychee checks links in this run.
Using warm medians, `gnd check .` is 5.9x faster than the configured Lychee run.
Per checked edge, `gnd` costs about 16 microseconds; Lychee costs about 217 microseconds per link.

Product copy:

> Lychee checks whether Markdown links still open. `gnd` checks whether the project still knows why the code exists.
> On this local run, `gnd` checks more than twice as many project-intent edges and finishes several times faster.

## Raw Warm Samples

- `gnd check`: 0.029s, 0.027s, 0.035s, 0.032s, 0.027s, 0.027s, 0.027s
- `gnd fmt --check`: 0.017s, 0.019s, 0.019s, 0.018s, 0.019s, 0.019s, 0.019s
- `lychee`: 0.159s, 0.162s, 0.193s, 0.155s, 0.182s, 0.169s, 0.157s
