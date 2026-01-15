use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use mysql_async::prelude::*;
use mysql_async::{Conn, Opts, OptsBuilder, Transaction, TxOpts};
use rlt::{bench_cli, bench_cli_run, BenchSuite, IterInfo, IterReport, Status};
use tokio::time::Instant;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TxMode {
    /// Auto-commit mode (no explicit transaction)
    AutoCommit,
    /// Optimistic transaction
    Optimistic,
    /// Pessimistic transaction
    Pessimistic,
}

bench_cli!(InsertBench, {
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

    /// Name of the table to insert into.
    #[clap(long, default_value = "bench_table")]
    pub table: String,

    /// Number of rows to insert in each batch.
    #[clap(long, short = 'b', default_value_t = 100)]
    pub batch_size: u32,

    /// Transaction mode: auto-commit, optimistic, or pessimistic
    #[clap(long, short = 'm', value_enum, default_value = "auto-commit")]
    pub tx_mode: TxMode,
});

pub struct WorkerState {
    conn: Conn,
    insert_counter: u64,
}

#[async_trait]
impl BenchSuite for InsertBench {
    type WorkerState = WorkerState;

    async fn state(&self, _worker_id: u32) -> Result<Self::WorkerState> {
        let opts = OptsBuilder::default()
            .ip_or_hostname(&self.host)
            .tcp_port(self.port)
            .user(Some(&self.user))
            .pass(Some(&self.password))
            .db_name(Some(&self.database));

        let conn = Conn::new(Opts::from(opts)).await?;
        Ok(WorkerState {
            conn,
            insert_counter: 0,
        })
    }

    async fn setup(&mut self, state: &mut Self::WorkerState, _worker_id: u32) -> Result<()> {
        // Drop table if exists (idempotent)
        state
            .conn
            .query_drop(format!("DROP TABLE IF EXISTS {}", self.table))
            .await?;

        // Create table
        state
            .conn
            .query_drop(format!(
                "CREATE TABLE {} (
                    id BIGINT PRIMARY KEY AUTO_INCREMENT,
                    data VARCHAR(255),
                    value INT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )",
                self.table
            ))
            .await?;

        Ok(())
    }

    async fn bench(
        &mut self,
        state: &mut Self::WorkerState,
        _info: &IterInfo,
    ) -> Result<IterReport> {
        let t = Instant::now();
        let mut bytes = 0u64;

        match self.tx_mode {
            TxMode::AutoCommit => {
                // Auto-commit: batch insert without explicit transaction
                let mut values = Vec::new();
                for i in 0..self.batch_size {
                    let counter = state.insert_counter + i as u64;
                    values.push(format!(
                        "('bench_data_{}', {})",
                        counter,
                        counter % 1000
                    ));
                }
                
                let query = format!(
                    "INSERT INTO {} (data, value) VALUES {}",
                    self.table,
                    values.join(", ")
                );
                
                state.conn.query_drop(&query).await?;
                
                // Approximate bytes: data string + int + overhead
                bytes = (self.batch_size as u64) * (50 + 4);
            }
            TxMode::Optimistic => {
                // Optimistic transaction
                let mut tx: Transaction<'_> = state.conn.start_transaction(TxOpts::default()).await?;
                
                let mut values = Vec::new();
                for i in 0..self.batch_size {
                    let counter = state.insert_counter + i as u64;
                    values.push(format!(
                        "('bench_data_{}', {})",
                        counter,
                        counter % 1000
                    ));
                }
                
                let query = format!(
                    "INSERT INTO {} (data, value) VALUES {}",
                    self.table,
                    values.join(", ")
                );
                
                tx.query_drop(&query).await?;
                tx.commit().await?;
                
                bytes = (self.batch_size as u64) * (50 + 4);
            }
            TxMode::Pessimistic => {
                // Pessimistic transaction: use tidb_txn_mode session variable
                state
                    .conn
                    .query_drop("SET SESSION tidb_txn_mode = 'pessimistic'")
                    .await?;
                
                let mut tx: Transaction<'_> = state.conn.start_transaction(TxOpts::default()).await?;
                
                let mut values = Vec::new();
                for i in 0..self.batch_size {
                    let counter = state.insert_counter + i as u64;
                    values.push(format!(
                        "('bench_data_{}', {})",
                        counter,
                        counter % 1000
                    ));
                }
                
                let query = format!(
                    "INSERT INTO {} (data, value) VALUES {}",
                    self.table,
                    values.join(", ")
                );
                
                tx.query_drop(&query).await?;
                tx.commit().await?;
                
                // Reset to default
                state
                    .conn
                    .query_drop("SET SESSION tidb_txn_mode = 'optimistic'")
                    .await?;
                
                bytes = (self.batch_size as u64) * (50 + 4);
            }
        }

        state.insert_counter += self.batch_size as u64;
        let duration = t.elapsed();

        Ok(IterReport {
            duration,
            status: Status::success(0),
            bytes,
            items: self.batch_size as u64,
        })
    }

    async fn teardown(self, mut state: Self::WorkerState, _info: IterInfo) -> Result<()> {
        // Clean up: drop the test table
        state
            .conn
            .query_drop(format!("DROP TABLE IF EXISTS {}", self.table))
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    bench_cli_run!(InsertBench).await
}
