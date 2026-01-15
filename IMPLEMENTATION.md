# Implementation Summary

This project implements a comprehensive TiDB benchmarking tool using the `rlt` load testing framework, following the requirements specified in the issue.

## Requirements Met

### 1. ✅ Separate Binaries for SELECT and INSERT
- `bench-select`: SELECT operation benchmark
- `bench-insert`: INSERT operation benchmark
- Each is an independent binary with its own configuration

### 2. ✅ MySQL 8 Protocol Support for TiDB
- Uses `mysql_async` crate version 0.34
- Compatible with MySQL 8 protocol that TiDB implements
- Supports all TiDB-specific features including transaction modes

### 3. ✅ Idempotent Tests
- **Setup phase**: Creates tables with `DROP TABLE IF EXISTS` to ensure clean state
- **Benchmark phase**: Performs operations with proper transaction handling
- **Teardown phase**: Cleans up test tables
- Tests can be run multiple times without conflicts

### 4. ✅ Transaction Mode Options
Each benchmark supports three transaction modes via `--tx-mode` flag:

#### Auto-commit (`--tx-mode auto-commit`)
- Operations run without explicit transactions
- Each query commits immediately
- Best for testing baseline performance

#### Optimistic (`--tx-mode optimistic`)
- Uses TiDB's default optimistic transaction mode
- Conflicts detected at commit time
- Good for workloads with low contention

#### Pessimistic (`--tx-mode pessimistic`)
- Sets `tidb_txn_mode='pessimistic'` session variable
- Locks acquired during transaction
- Better for high-contention scenarios

### 5. ✅ Byte Size Tracking
- **SELECT**: Tracks actual data size by calculating `BIGINT_SIZE + string.len()` for each row
- **INSERT**: Estimates data size using constants `AVG_STRING_DATA_SIZE + INT_VALUE_SIZE`
- Reported in `IterReport.bytes` field
- Enables bandwidth calculation in TUI (MB/s)

## Architecture

### SELECT Benchmark Flow
```
Setup → Create Table → Insert Test Data (2x select count)
  ↓
Bench → SELECT with chosen transaction mode → Track bytes
  ↓
Teardown → Drop Table
```

### INSERT Benchmark Flow
```
Setup → Create Table
  ↓
Bench → Batch INSERT with chosen transaction mode → Track bytes
  ↓
Teardown → Drop Table
```

## Key Features

- **Real-time TUI**: Live statistics with rlt's built-in TUI
- **Configurable concurrency**: `-c` flag for worker count
- **Duration or iteration based**: `-d` for time, `-n` for iterations
- **Rate limiting**: `-r` flag for controlled throughput
- **Warmup support**: `-w` flag for warmup iterations
- **Multiple output formats**: Text or JSON (`-o json`)
- **Flexible configuration**: Host, port, database, table name all configurable

## Dependencies

- `rlt` (from git main branch): Core load testing framework
- `mysql_async`: MySQL 8 protocol support
- `tokio`: Async runtime
- `clap`: CLI argument parsing
- `anyhow`: Error handling
- `async-trait`: Async trait support

## Code Quality

- No compiler warnings
- Named constants instead of magic numbers
- Clear comments explaining transaction behavior
- Proper error handling with `Result<T>`
- Security: No SQL injection risk (all values programmatically generated)

## Usage Examples

See `examples/README.md` for detailed usage examples including:
- Basic benchmarking
- Transaction mode comparisons
- High concurrency testing
- Rate-limited testing
