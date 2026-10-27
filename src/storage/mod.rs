use std::path::Path;

pub fn get_default_path() -> String {
    Path::new(&std::env::var("HOME").unwrap()).join(".nebula").to_string_lossy().to_string()
}

mod db;

pub use db::{Database, DatabaseError};