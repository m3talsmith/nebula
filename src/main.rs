use anyhow::{Result, Error};

mod storage;
use crate::storage::{Database, DatabaseError, get_default_path};

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> Result<(), Error> {
    let _: Database = Database::new(get_default_path()).await.map_err(|e| Error::new(e))?;
    println!("Database opened");
    Ok(())
}