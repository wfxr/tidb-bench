# Examples

## Running Benchmarks Against TiDB

### Prerequisites

Make sure you have a TiDB server running. You can start one locally using:

```bash
# Using TiUP (recommended)
tiup playground

# Or using Docker
docker run -d --name tidb -p 4000:4000 pingcap/tidb:latest
```

### Example 1: Basic SELECT Benchmark

Test SELECT performance with default settings:

```bash
./target/release/bench-select \
  --host 127.0.0.1 \
  --port 4000 \
  -d 10s
```

### Example 2: INSERT Benchmark with Pessimistic Transactions

Test INSERT performance using pessimistic transactions:

```bash
./target/release/bench-insert \
  --host 127.0.0.1 \
  --port 4000 \
  --tx-mode pessimistic \
  --batch-size 1000 \
  -c 5 \
  -d 30s
```

### Example 3: Comparison of Transaction Modes

Run the same benchmark with different transaction modes to compare:

```bash
# Auto-commit mode
./target/release/bench-insert --tx-mode auto-commit -n 10000 -q -o json > results-autocommit.json

# Optimistic mode
./target/release/bench-insert --tx-mode optimistic -n 10000 -q -o json > results-optimistic.json

# Pessimistic mode
./target/release/bench-insert --tx-mode pessimistic -n 10000 -q -o json > results-pessimistic.json
```

### Example 4: High Concurrency SELECT Test

Test SELECT performance with high concurrency:

```bash
./target/release/bench-select \
  --host 127.0.0.1 \
  --port 4000 \
  --select-count 10000 \
  -c 50 \
  -d 1m \
  --tx-mode optimistic
```

### Example 5: Rate-Limited INSERT Test

Test INSERT performance with rate limiting:

```bash
./target/release/bench-insert \
  --host 127.0.0.1 \
  --port 4000 \
  --batch-size 500 \
  -r 100 \
  -d 1m
```

## Understanding the Output

The TUI will show real-time statistics including:

- **Throughput**: Operations per second
- **Bandwidth**: Data processed per second
- **Latency percentiles**: P50, P90, P95, P99
- **Success/Failure rates**: Number and percentage of successful operations
- **Progress**: Current iteration count and elapsed time

## Tips

1. **Warmup**: Use `-w` to specify warmup iterations that won't be included in final stats
2. **Quiet mode**: Use `-q` for automated benchmarking without TUI
3. **JSON output**: Use `-o json` for machine-readable results
4. **Custom table**: Use `--table` to specify a different table name if needed
