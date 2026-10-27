use std::path::{self, Path, PathBuf};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use chrono::Utc;
use std::error::Error;
use std::fmt;

const DEFAULT_NAME: &str = "nebula.db";

mod migrations {
    use sqlx::{migrate, SqlitePool};
    pub async fn run(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        migrate!("./migrations").run(pool).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum InvalidPathError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for InvalidPathError {
    fn from(e: std::io::Error) -> Self {
        InvalidPathError::IoError(e)
    }
}

impl Error for InvalidPathError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            InvalidPathError::IoError(e) => Some(e),
        }
    }
}

impl fmt::Display for InvalidPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidPathError::IoError(e) => write!(f, "io error: {:?}", e),
        }
    }
}

#[derive(Debug)]
pub enum RestoreError {
    IoError(std::io::Error),
    InvalidPathError(InvalidPathError),
}

impl From<std::io::Error> for RestoreError {
    fn from(e: std::io::Error) -> Self {
        RestoreError::IoError(e)
    }
}

impl From<InvalidPathError> for RestoreError {
    fn from(e: InvalidPathError) -> Self {
        RestoreError::InvalidPathError(e)
    }
}

impl Error for RestoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RestoreError::IoError(e) => Some(e),
            RestoreError::InvalidPathError(e) => Some(e),
        }
    }
}

impl fmt::Display for RestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RestoreError::IoError(e) => write!(f, "io error: {:?}", e),
            RestoreError::InvalidPathError(e) => write!(f, "invalid path error: {e}"),
        }
    }
}

#[derive(Debug)]
pub enum BackupError {
    SqlxError(sqlx::Error),
    IoError(std::io::Error),
    InvalidPathError(InvalidPathError),
}

impl From<sqlx::Error> for BackupError {
    fn from(e: sqlx::Error) -> Self {
        BackupError::SqlxError(e)
    }
}

impl From<std::io::Error> for BackupError {
    fn from(e: std::io::Error) -> Self {
        BackupError::IoError(e)
    }
}

impl Error for BackupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BackupError::SqlxError(e) => Some(e),
            BackupError::IoError(e) => Some(e),
            BackupError::InvalidPathError(e) => Some(e),
        }
    }
}

impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackupError::SqlxError(e) => write!(f, "sqlx error: {:?}", e),
            BackupError::IoError(e) => write!(f, "io error: {:?}", e),
            BackupError::InvalidPathError(e) => write!(f, "invalid path error: {e}"),
        }
    }
}

impl From<InvalidPathError> for BackupError {
    fn from(e: InvalidPathError) -> Self {
        BackupError::InvalidPathError(e)
    }
}

#[derive(Debug)]
pub enum DatabaseError {
    IoError(std::io::Error),
    SqlxError(sqlx::Error),
    BackupError(BackupError),
    InvalidPathError(InvalidPathError),
    RestoreError(RestoreError),
}

impl From<BackupError> for DatabaseError {
    fn from(e: BackupError) -> Self {
        DatabaseError::BackupError(e)
    }
}

impl From<RestoreError> for DatabaseError {
    fn from(e: RestoreError) -> Self {
        DatabaseError::RestoreError(e)
    }
}

impl From<std::io::Error> for DatabaseError {
    fn from(e: std::io::Error) -> Self {
        DatabaseError::IoError(e)
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(e: sqlx::Error) -> Self {
        DatabaseError::SqlxError(e)
    }
}

impl From<InvalidPathError> for DatabaseError {
    fn from(e: InvalidPathError) -> Self {
        DatabaseError::InvalidPathError(e)
    }
}

impl Error for DatabaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DatabaseError::IoError(e) => Some(e),
            DatabaseError::SqlxError(e) => Some(e),
            DatabaseError::BackupError(e) => Some(e),
            DatabaseError::InvalidPathError(e) => Some(e),
            DatabaseError::RestoreError(e) => Some(e),
        }
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::IoError(e) => write!(f, "io error: {e}"),
            DatabaseError::SqlxError(e) => write!(f, "sqlx error: {e}"),
            DatabaseError::BackupError(e) => write!(f, "backup error: {e}"),
            DatabaseError::InvalidPathError(e) => write!(f, "invalid path error: {e}"),
            DatabaseError::RestoreError(e) => write!(f, "restore error: {e}"),
        }
    }
}

pub struct Database {
    path: String,
    name: String,
    pool: SqlitePool,
}

impl Database {
    pub async fn new(path: String) -> Result<Self, DatabaseError> {
        if !Path::new(&path).exists() {
            std::fs::create_dir_all(&path).map_err(|e| DatabaseError::IoError(e))?;

        }
        let options = SqliteConnectOptions::new()
        .filename(&Path::new(&path).join(&DEFAULT_NAME).to_string_lossy().to_string())
        .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await.map_err(|e| DatabaseError::SqlxError(e))?;
        migrations::run(&pool).await.map_err(|e| DatabaseError::SqlxError(e))?;
        Ok(Self { path, name: DEFAULT_NAME.to_string(), pool })
    }

    fn database_path(&self) -> PathBuf {
        Path::new(&self.path).join(&self.name)
    }

    pub fn backup(&self, backup_to_path: String) -> Result<(), DatabaseError> {
        let backup = Backup::new(backup_to_path, self.database_path().to_string_lossy().to_string()).map_err(|e| DatabaseError::BackupError(e))?;
        backup.backup().map_err(|e| DatabaseError::BackupError(e))?;
        Ok(())
    }

    pub fn restore(&self, restore_from_path: String) -> Result<(), DatabaseError> {
        let backup = Backup::new(self.database_path().to_string_lossy().to_string(), restore_from_path).map_err(|e| DatabaseError::BackupError(e))?;
        backup.restore().map_err(|e| DatabaseError::RestoreError(e))?;
        Ok(())
    }
}

pub struct Backup {
    to_path: String,
    from_path: String,
}

impl Backup {
    pub fn new(to_path: String, from_path: String) -> Result<Self, BackupError> {
        if Self::validate_path(&from_path).is_err() {
            return Err(BackupError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, format!("From path ( {} ) does not exist", from_path))));
        }
        if Self::validate_path(&to_path).is_err() {
            std::fs::create_dir_all(&Path::new(&to_path)).map_err(|e| BackupError::IoError(e))?;  
        }
        Ok(Self { to_path, from_path })
    }

    fn validate_path(path: &String) -> Result<(), BackupError> {
        if !Path::new(path).exists() {
            return Err(BackupError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, format!("To path ( {} ) does not exist", path))));
        }
        Ok(())
    }

    pub fn backup(&self) -> Result<(), BackupError> {
        let (from_name, from_path) = match Self::get_name_and_path(self.from_path.clone()) {
            Ok((name, path)) => (name, path),
            Err(e) => return Err(BackupError::InvalidPathError(e)),
        };
        let (to_name, to_path) = match Self::get_name_and_path(self.to_path.clone()) {
            Ok((name, path)) => (name, path),
            Err(e) => return Err(BackupError::InvalidPathError(e)),
        };
        let from_file_path = from_path.join(&from_name);
        let to_file_path = to_path.join(format!("{}-{}.backup", to_name, Utc::now().to_rfc3339()));
        std::fs::copy(&from_file_path, &to_file_path).map_err(|e| BackupError::IoError(e))?;
        Ok(())
    }

    pub fn restore(&self) -> Result<(), RestoreError> {
        let (from_name, from_path) = match Self::get_name_and_path(self.from_path.clone()) {
            Ok((name, path)) => (name, path),
            Err(e) => return Err(RestoreError::InvalidPathError(e)),
        };
        let (to_name, to_path) = match Self::get_name_and_path(self.to_path.clone()) {
            Ok((name, path)) => (name, path),
            Err(e) => return Err(RestoreError::InvalidPathError(e)),
        };
        let from_file_path = from_path.join(&from_name);
        let to_file_path = to_path.join(to_name);
        std::fs::copy(from_file_path, &to_file_path).map_err(|e| RestoreError::IoError(e))?;
        Ok(())
    }

    fn get_name_and_path(full_path: String) -> Result<(String, PathBuf), InvalidPathError> {
        let split = full_path.split(path::MAIN_SEPARATOR).collect::<Vec<&str>>().into_iter().rev().collect::<Vec<&str>>();
        let (db_name, dir_path) = match &split[..] {
            [db_name, rest @ ..] => {
                let dir_path = Path::new(rest.join(&path::MAIN_SEPARATOR.to_string().as_str()).as_str()).to_path_buf();
                (db_name.to_string(), dir_path)
            },
            _ => {
                let error = InvalidPathError::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"));   
                return Err(error);
            }
        };
        Ok((db_name, dir_path))
    }
}