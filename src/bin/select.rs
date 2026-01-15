use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use mysql_async::prelude::*;
use mysql_async::{Conn, Opts, OptsBuilder, Transaction, TxOpts};
use rlt::{bench_cli, bench_cli_run, BenchSuite, IterInfo, IterReport, Status};
use tokio::time::Instant;

// Configuration constants
const TEST_DATA_MULTIPLIER: u32 = 2; // Insert 2x more rows than we'll select
const BIGINT_SIZE: u64 = 8;          // Size of BIGINT column in bytes

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TxMode {
    /// Auto-commit mode (no explicit transaction)
    AutoCommit,
    /// Optimistic transaction
    Optimistic,
    /// Pessimistic transaction
    Pessimistic,
}

bench_cli!(SelectBench, {
    /// Host of the TiDB server.
    #[clap(long, default_value = "localhost")]
    pub host: String,

    /// Port of the TiDB server.
    #[clap(long, default_value_t = 3306)]
    pub port: u16,

    /// Username for authentication.
    #[clap(long, default_value = "root")]
    pub user: String,

    /// Password for authentication.
    #[clap(long, default_value = "")]
    pub password: String,

    /// Database name.
    #[clap(long, default_value = "test")]
    pub database: String,

    /// Name of the table to select from.
    #[clap(long, default_value = "bench_table")]
    pub table: String,

    /// Number of rows to select in each iteration.
    #[clap(long, default_value_t = 1000)]
    pub select_count: u32,

    /// Transaction mode: auto-commit, optimistic, or pessimistic
    #[clap(long, short = 'm', value_enum, default_value = "auto-commit")]
    pub tx_mode: TxMode,
});

#[async_trait]
impl BenchSuite for SelectBench {
    type WorkerState = Conn;

    async fn state(&self, _worker_id: u32) -> Result<Self::WorkerState> {
        let opts = OptsBuilder::default()
            .ip_or_hostname(&self.host)
            .tcp_port(self.port)
            .user(Some(&self.user))
            .pass(Some(&self.password))
            .db_name(Some(&self.database));

        let conn = Conn::new(Opts::from(opts)).await?;
        Ok(conn)
    }

    async fn setup(&mut self, conn: &mut Self::WorkerState, _worker_id: u32) -> Result<()> {
        // Drop table if exists (idempotent)
        conn.query_drop(format!("DROP TABLE IF EXISTS {}", self.table))
            .await?;

        // Create table
        conn.query_drop(format!(
            "CREATE TABLE {} (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                data VARCHAR(255),
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            self.table
        ))
        .await?;

        // Insert test data (insert more than we'll select to ensure enough data)
        let insert_count = self.select_count * TEST_DATA_MULTIPLIER;
        conn.exec_drop(
            format!(
                "INSERT INTO {} (data) 
                 SELECT CONCAT('test_data_', n) 
                 FROM (
                   SELECT @row := @row + 1 as n 
                   FROM (SELECT 0 UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9) t1,
                        (SELECT 0 UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9) t2,
                        (SELECT 0 UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9) t3,
                        (SELECT 0 UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9) t4,
                        (SELECT @row := 0) r
                 ) nums 
                 WHERE n <= ?",
                self.table
            ),
            (insert_count,),
        )
        .await?;

        Ok(())
    }

    async fn bench(&mut self, conn: &mut Self::WorkerState, _info: &IterInfo) -> Result<IterReport> {
        let t = Instant::now();
        let mut bytes = 0u64;

        match self.tx_mode {
            TxMode::AutoCommit => {
                // Auto-commit: just run the query directly
                let result: Vec<(i64, String)> = conn
                    .exec(
                        format!("SELECT id, data FROM {} LIMIT ?", self.table),
                        (self.select_count,),
                    )
                    .await?;
                
                // Calculate approximate bytes
                for (_, data) in &result {
                    bytes += BIGINT_SIZE + data.len() as u64;
                }
            }
            TxMode::Optimistic => {
                // Optimistic transaction
                let mut tx: Transaction<'_> = conn.start_transaction(TxOpts::default()).await?;
                
                let result: Vec<(i64, String)> = tx
                    .exec(
                        format!("SELECT id, data FROM {} LIMIT ?", self.table),
                        (self.select_count,),
                    )
                    .await?;
                
                // Calculate approximate bytes
                for (_, data) in &result {
                    bytes += BIGINT_SIZE + data.len() as u64;
                }
                
                tx.commit().await?;
            }
            TxMode::Pessimistic => {
                // Pessimistic transaction: use tidb_txn_mode session variable
                conn.query_drop("SET SESSION tidb_txn_mode = 'pessimistic'")
                    .await?;
                
                let mut tx: Transaction<'_> = conn.start_transaction(TxOpts::default()).await?;
                
                let result: Vec<(i64, String)> = tx
                    .exec(
                        format!("SELECT id, data FROM {} LIMIT ?", self.table),
                        (self.select_count,),
                    )
                    .await?;
                
                // Calculate approximate bytes
                for (_, data) in &result {
                    bytes += BIGINT_SIZE + data.len() as u64;
                }
                
                tx.commit().await?;
                
                // Reset to default
                conn.query_drop("SET SESSION tidb_txn_mode = 'optimistic'")
                    .await?;
            }
        }

        let duration = t.elapsed();

        Ok(IterReport {
            duration,
            status: Status::success(0),
            bytes,
            items: self.select_count as u64,
        })
    }

    async fn teardown(self, mut conn: Self::WorkerState, _info: IterInfo) -> Result<()> {
        // Clean up: drop the test table
        conn.query_drop(format!("DROP TABLE IF EXISTS {}", self.table))
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    bench_cli_run!(SelectBench).await
}
