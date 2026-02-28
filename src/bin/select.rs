use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use mysql_async::prelude::*;
use mysql_async::{Conn, TxOpts};
use rand::Rng;
use rlt::{BenchSuite, IterInfo, IterReport, Status};
use tidb_bench::{DbOpts, TxMode};
use tokio::sync::Barrier;
use tokio::time::Instant;

const BIGINT_SIZE: u64 = 8;
const TEST_DATA_MULTIPLIER: u32 = 2;
const INSERT_BATCH_SIZE: u32 = 5000;

/// TiDB SELECT benchmark.
#[derive(Parser, Clone)]
struct SelectCli {
    #[command(flatten)]
    db: DbOpts,

    /// Number of rows to select per query.
    #[clap(long, default_value_t = 1000)]
    select_count: u32,

    #[command(flatten)]
    bench_opts: rlt::cli::BenchCli,
}

#[derive(Clone)]
struct SelectBench {
    db: DbOpts,
    select_count: u32,
    total_rows: u32,
    barrier: Arc<Barrier>,
}

impl SelectBench {
    fn from_cli(cli: &SelectCli) -> Self {
        Self {
            db: cli.db.clone(),
            select_count: cli.select_count,
            total_rows: cli.select_count * TEST_DATA_MULTIPLIER,
            barrier: Arc::new(Barrier::new(cli.bench_opts.concurrency.get() as usize)),
        }
    }

    /// Insert test rows in batches.
    async fn insert_test_data(&self, conn: &mut Conn) -> Result<()> {
        let table = self.db.quoted_table();
        for start in (0..self.total_rows).step_by(INSERT_BATCH_SIZE as usize) {
            let end = (start + INSERT_BATCH_SIZE).min(self.total_rows);
            let values = (start..end)
                .map(|i| format!("('test_data_{i}')"))
                .collect::<Vec<_>>()
                .join(", ");
            conn.query_drop(format!("INSERT INTO {table} (data) VALUES {values}"))
                .await?;
        }
        Ok(())
    }

    fn max_offset(&self) -> u32 {
        self.total_rows.saturating_sub(self.select_count)
    }
}

#[async_trait]
impl BenchSuite for SelectBench {
    type WorkerState = Conn;

    async fn setup(&mut self, worker_id: u32) -> Result<Self::WorkerState> {
        let mut conn = self.db.connect().await?;
        self.db.init_tx_mode(&mut conn).await?;

        if worker_id == 0 {
            let table = self.db.quoted_table();
            conn.query_drop(format!("DROP TABLE IF EXISTS {table}"))
                .await?;
            conn.query_drop(format!(
                "CREATE TABLE {table} (
                    id BIGINT PRIMARY KEY AUTO_INCREMENT,
                    data VARCHAR(255),
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )"
            ))
            .await?;
            self.insert_test_data(&mut conn).await?;
        }

        self.barrier.wait().await;
        Ok(conn)
    }

    async fn bench(&mut self, conn: &mut Conn, _info: &IterInfo) -> Result<IterReport> {
        let t = Instant::now();
        let table = self.db.quoted_table();
        let offset = rand::thread_rng().gen_range(0..=self.max_offset());
        let query = format!(
            "SELECT id, data FROM {table} LIMIT {} OFFSET {offset}",
            self.select_count
        );

        let result: Vec<(i64, String)> = match self.db.tx_mode {
            TxMode::AutoCommit => conn.query(&query).await?,
            TxMode::Optimistic | TxMode::Pessimistic => {
                let mut tx = conn.start_transaction(TxOpts::default()).await?;
                let rows = tx.query(&query).await?;
                tx.commit().await?;
                rows
            }
        };

        let bytes: u64 = result
            .iter()
            .map(|(_, data)| BIGINT_SIZE + data.len() as u64)
            .sum();

        Ok(IterReport {
            duration: t.elapsed(),
            status: Status::success(0),
            bytes,
            items: self.select_count as u64,
        })
    }

    async fn teardown(self, mut conn: Conn, info: IterInfo) -> Result<()> {
        if info.worker_id == 0 {
            conn.query_drop(format!("DROP TABLE IF EXISTS {}", self.db.quoted_table()))
                .await?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = SelectCli::parse();
    let bench = SelectBench::from_cli(&cli);
    rlt::cli::run(cli.bench_opts, bench).await?;
    Ok(())
}
