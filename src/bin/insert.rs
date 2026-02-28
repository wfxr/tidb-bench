use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use mysql_async::prelude::*;
use mysql_async::{Conn, TxOpts};
use rlt::{BenchSuite, IterInfo, IterReport, Status};
use tidb_bench::{DbOpts, TxMode};
use tokio::sync::Barrier;
use tokio::time::Instant;

const AVG_ROW_SIZE: u64 = 54; // ~50 bytes string + 4 bytes int

/// TiDB INSERT benchmark.
#[derive(Parser, Clone)]
struct InsertCli {
    #[command(flatten)]
    db: DbOpts,

    /// Number of rows to insert per batch.
    #[clap(long, short = 'b', default_value_t = 100)]
    batch_size: u32,

    #[command(flatten)]
    bench_opts: rlt::cli::BenchCli,
}

#[derive(Clone)]
struct InsertBench {
    db: DbOpts,
    batch_size: u32,
    barrier: Arc<Barrier>,
}

impl InsertBench {
    fn from_cli(cli: &InsertCli) -> Self {
        Self {
            db: cli.db.clone(),
            batch_size: cli.batch_size,
            barrier: Arc::new(Barrier::new(cli.bench_opts.concurrency.get() as usize)),
        }
    }

    fn build_batch_values(&self, counter: u64) -> String {
        (0..self.batch_size)
            .map(|i| {
                let c = counter + i as u64;
                format!("('bench_data_{c}', {})", c % 1000)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[async_trait]
impl BenchSuite for InsertBench {
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
                    value INT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )"
            ))
            .await?;
        }

        self.barrier.wait().await;
        Ok(conn)
    }

    async fn bench(&mut self, conn: &mut Conn, info: &IterInfo) -> Result<IterReport> {
        let t = Instant::now();
        let counter = info.worker_seq * self.batch_size as u64;
        let table = self.db.quoted_table();
        let values = self.build_batch_values(counter);
        let query = format!("INSERT INTO {table} (data, value) VALUES {values}");

        match self.db.tx_mode {
            TxMode::AutoCommit => {
                conn.query_drop(&query).await?;
            }
            TxMode::Optimistic | TxMode::Pessimistic => {
                let mut tx = conn.start_transaction(TxOpts::default()).await?;
                tx.query_drop(&query).await?;
                tx.commit().await?;
            }
        }

        Ok(IterReport {
            duration: t.elapsed(),
            status: Status::success(0),
            bytes: self.batch_size as u64 * AVG_ROW_SIZE,
            items: self.batch_size as u64,
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
    let cli = InsertCli::parse();
    let bench = InsertBench::from_cli(&cli);
    rlt::cli::run(cli.bench_opts, bench).await?;
    Ok(())
}
