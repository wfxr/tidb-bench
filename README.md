# tidb-bench

A performance benchmarking tool for TiDB database using the [rlt](https://github.com/wfxr/rlt) load testing framework.

## Features

- **Separate Binaries**: Independent benchmark binaries for SELECT and INSERT operations
- **Transaction Modes**: Support for three transaction modes:
  - `auto-commit`: Direct operations without explicit transactions
  - `optimistic`: TiDB optimistic transactions
  - `pessimistic`: TiDB pessimistic transactions
- **Idempotent Tests**: Proper setup and teardown ensure tests can be run repeatedly
- **Real-time TUI**: Beautiful terminal UI showing live benchmark statistics
- **Flexible Options**: Configurable concurrency, duration, rate limiting, and more
- **Byte Tracking**: Reports approximate data size processed in each operation

## Prerequisites

- Rust toolchain (1.70+)
- TiDB server (or MySQL 8+ compatible database)

## Building

```bash
cargo build --release
```

This creates two binaries:
- `target/release/bench-select` - SELECT operation benchmark
- `target/release/bench-insert` - INSERT operation benchmark

## Usage

### SELECT Benchmark

```bash
# Basic usage with default settings (localhost:3306)
./target/release/bench-select

# Specify TiDB connection
./target/release/bench-select \
  --host 127.0.0.1 \
  --port 4000 \
  --user root \
  --password "" \
  --database test

# Run with pessimistic transactions
./target/release/bench-select --tx-mode pessimistic

# Run 10 concurrent workers for 30 seconds
./target/release/bench-select -c 10 -d 30s

# Select 5000 rows per iteration
./target/release/bench-select --select-count 5000

# Limit to 100 requests per second
./target/release/bench-select -r 100
```

### INSERT Benchmark

```bash
# Basic usage with default settings
./target/release/bench-insert

# Insert 500 rows per batch with optimistic transactions
./target/release/bench-insert \
  --batch-size 500 \
  --tx-mode optimistic

# Run 5 concurrent workers for 1 minute
./target/release/bench-insert -c 5 -d 1m

# Run exactly 1000 iterations
./target/release/bench-insert -n 1000

# Output results as JSON
./target/release/bench-insert -o json
```

## Transaction Modes

### Auto-commit
```bash
--tx-mode auto-commit
```
Operations run without explicit transactions. Each query commits immediately.

### Optimistic
```bash
--tx-mode optimistic
```
Uses TiDB's default optimistic transaction mode. Conflicts are detected at commit time.

### Pessimistic
```bash
--tx-mode pessimistic
```
Uses TiDB's pessimistic transaction mode by setting `tidb_txn_mode='pessimistic'`. Locks are acquired during the transaction.

## Common Options

All benchmarks support these common options from rlt:

- `-c, --concurrency <N>`: Number of concurrent workers (default: 1)
- `-n, --iterations <N>`: Total number of iterations to run
- `-d, --duration <TIME>`: Duration to run (e.g., 10s, 5m, 1h)
- `-w, --warmup <N>`: Number of warmup iterations (default: 0)
- `-r, --rate <N>`: Rate limit in iterations per second
- `-q, --quiet`: Run in quiet mode (no TUI)
- `-o, --output <FORMAT>`: Output format (text or json)
- `-O, --output-file <PATH>`: Write report to file

## Architecture

### SELECT Benchmark (`bench-select`)

1. **Setup**: Creates a test table and populates it with data
2. **Benchmark**: Performs SELECT queries with configurable transaction mode
3. **Teardown**: Drops the test table

The benchmark tracks:
- Query duration
- Number of rows selected
- Approximate bytes transferred (row data size)

### INSERT Benchmark (`bench-insert`)

1. **Setup**: Creates a test table
2. **Benchmark**: Performs batch INSERT operations with configurable transaction mode
3. **Teardown**: Drops the test table

The benchmark tracks:
- Insert duration
- Number of rows inserted
- Approximate bytes written

## Implementation Details

- **MySQL Protocol**: Uses `mysql_async` crate for MySQL 8 protocol support (TiDB compatible)
- **Async Runtime**: Built on Tokio for high-performance async I/O
- **Idempotency**: Tables are dropped and recreated in setup, ensuring clean state
- **Load Testing Framework**: Built on `rlt` for professional-grade load testing with real-time TUI

## Example Output

```
┌─ Benchmark Summary ────────────────────────────────────────────────────┐
│ Total:     10000 iterations                                             │
│ Duration:  30.5s                                                        │
│ Success:   10000 (100.00%)                                             │
│ Failed:    0 (0.00%)                                                   │
│                                                                         │
│ Throughput: 327.87 ips                                                 │
│ Bandwidth:  10.2 MB/s                                                  │
│                                                                         │
│ Latency (ms):                                                          │
│   Min:     2.45                                                        │
│   P50:     3.12                                                        │
│   P90:     4.23                                                        │
│   P99:     7.89                                                        │
│   Max:     15.67                                                       │
└────────────────────────────────────────────────────────────────────────┘
```

## Development

The project structure:
```
tidb-bench/
├── Cargo.toml          # Project dependencies
├── src/
│   └── bin/
│       ├── select.rs   # SELECT benchmark
│       └── insert.rs   # INSERT benchmark
└── README.md
```

Each binary is independent and implements the `BenchSuite` trait from rlt.

## License

This project follows the same license as the dependencies it uses.