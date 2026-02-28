use anyhow::Result;
use mysql_async::prelude::*;
use mysql_async::{Conn, Opts, OptsBuilder};

pub const DEFAULT_PORT: u16 = 4000;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TxMode {
    /// No explicit transaction; each query auto-commits.
    AutoCommit,
    /// TiDB optimistic transactions.
    Optimistic,
    /// TiDB pessimistic transactions.
    Pessimistic,
}

/// Common database connection and benchmark options.
#[derive(clap::Args, Clone)]
pub struct DbOpts {
    /// TiDB server host.
    #[clap(long, default_value = "localhost")]
    pub host: String,

    /// TiDB server port.
    #[clap(long, default_value_t = DEFAULT_PORT)]
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

    /// Benchmark table name.
    #[clap(long, default_value = "bench_table")]
    pub table: String,

    /// Transaction mode.
    #[clap(long, short = 'm', value_enum, default_value = "auto-commit")]
    pub tx_mode: TxMode,
}

impl DbOpts {
    pub async fn connect(&self) -> Result<Conn> {
        let opts = OptsBuilder::default()
            .ip_or_hostname(&self.host)
            .tcp_port(self.port)
            .user(Some(&self.user))
            .pass(Some(&self.password))
            .db_name(Some(&self.database));
        Ok(Conn::new(Opts::from(opts)).await?)
    }

    /// Set TiDB transaction mode for the session (once per connection).
    pub async fn init_tx_mode(&self, conn: &mut Conn) -> Result<()> {
        match self.tx_mode {
            TxMode::AutoCommit => {}
            TxMode::Optimistic => {
                conn.query_drop("SET SESSION tidb_txn_mode = 'optimistic'")
                    .await?;
            }
            TxMode::Pessimistic => {
                conn.query_drop("SET SESSION tidb_txn_mode = 'pessimistic'")
                    .await?;
            }
        }
        Ok(())
    }

    pub fn quoted_table(&self) -> String {
        format!("`{}`", self.table)
    }
}
