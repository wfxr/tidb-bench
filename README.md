# tidb-bench

Benchmark tool for TiDB using [rlt](https://github.com/wfxr/rlt).

## Prerequisites

- Rust toolchain (1.70+)
- A running TiDB server (e.g. `tiup playground` or `docker run -p 4000:4000 pingcap/tidb`)

## Build

```bash
cargo build --release
```

## Usage

```bash
# SELECT: 4 workers, 30 seconds, pessimistic transactions
bench-select -c 4 -d 30s --tx-mode pessimistic

# INSERT: 8 workers, 500 rows/batch, optimistic transactions
bench-insert -c 8 -d 1m --tx-mode optimistic -b 500

# Custom connection
bench-select --host 10.0.0.1 --port 4000 --user root --database mydb

# JSON output for scripting
bench-insert -c 4 -d 10s -q -o json

# Compare transaction modes
for mode in auto-commit optimistic pessimistic; do
  bench-insert -c 4 -d 10s --tx-mode $mode -q -o json > "result-$mode.json"
done
```

## CLI Options

### Connection

| Option | Default | Description |
|--------|---------|-------------|
| `--host` | `localhost` | TiDB server host |
| `--port` | `4000` | TiDB server port |
| `--user` | `root` | Username |
| `--password` | `""` | Password |
| `--database` | `test` | Database name |
| `--table` | `bench_table` | Benchmark table name |
| `-m, --tx-mode` | `auto-commit` | Transaction mode (see below) |

### Transaction Modes

| Mode | Behavior |
|------|----------|
| `auto-commit` | No explicit transaction; each query auto-commits |
| `optimistic` | Conflicts detected at commit time |
| `pessimistic` | Locks acquired during execution |

The mode is set once per connection via `SET SESSION tidb_txn_mode`.

### Benchmark-Specific

| Option | Binary | Default | Description |
|--------|--------|---------|-------------|
| `--select-count` | `bench-select` | `1000` | Rows per SELECT query |
| `-b, --batch-size` | `bench-insert` | `100` | Rows per INSERT batch |

### Load Control (from rlt)

| Option | Description |
|--------|-------------|
| `-c, --concurrency <N>` | Concurrent workers (default: 1) |
| `-n, --iterations <N>` | Total iterations |
| `-d, --duration <TIME>` | Run duration (e.g. `10s`, `5m`, `1h`) |
| `-w, --warmup <N>` | Warmup iterations (default: 0) |
| `-r, --rate <N>` | Rate limit (iterations/sec) |
| `-q, --quiet` | Suppress TUI |
| `-o, --output <FMT>` | Output format: `text` or `json` |
| `-O, --output-file <PATH>` | Write report to file |

## How It Works

1. **Setup** — Worker 0 creates (or recreates) the benchmark table; SELECT also pre-populates test data. All workers synchronize via a barrier before benchmarking begins.
2. **Bench** — Each worker runs queries in a loop. Transaction mode is set once per connection, not per iteration.
3. **Teardown** — Worker 0 drops the table.

## Project Structure

```
src/
├── lib.rs        # Shared types: DbOpts, TxMode
└── bin/
    ├── select.rs # bench-select
    └── insert.rs # bench-insert
```
