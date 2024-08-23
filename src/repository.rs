use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::io::Write;

use axum::{extract::State, response::IntoResponse, response::Json};

use crate::database;
use std::sync::Arc;
use std::{io, io::Error, io::ErrorKind::InvalidData};
use derive_more::Display;

#[derive(thiserror::Error, Debug)]
pub enum RepositoryError {
    #[error("Could not create repository, database error:{0}")]
    CouldNotCreateRepository(#[from] database::DatabaseError),

    #[error("Could not read configuration:{0}, io::error:{1}")]
    CouldNotReadConfiguration(String, io::Error),

    #[error("Could not decode configuration:{0}, io::error:{1}")]
    CouldNotDecodeConfiguration(String, serde_yaml::Error)
}

use RepositoryError::{CouldNotReadConfiguration, CouldNotDecodeConfiguration};

#[derive(Debug, Clone)]
pub struct Repository {
    pub config: RepositoryConfig,
    pub db_conn: database::Pool,
}

impl Repository {
    pub fn new(config_path: &str) -> Result<Repository, RepositoryError> {
        let config = RepositoryConfig::new(&config_path)?;
        let db_conn = database::init_db_pool_connection(&config.db_file)?;
        Ok(Repository { config, db_conn })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepositoryConfig {
    pub db_file: String,
    pub uploads_dir: String,
    pub pool_dir: String,
    pub dists: HashMap<String, Distribution>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Display)]
#[display(fmt = "Distribution: {:#?}", self)]
pub struct Distribution {
    pub origin: String,
    pub label: String,
    pub version: String,
    pub codename: String,
    pub description: String,
    pub components: Vec<String>,
    pub architectures: Vec<String>,
}

impl RepositoryConfig {
    pub fn new(config_path: &str) -> Result<RepositoryConfig, RepositoryError> {
        let yaml_content = read_to_string(config_path).map_err(|err| {CouldNotReadConfiguration(config_path.to_string(), err)})?;
        let archive: RepositoryConfig = serde_yaml::from_str(&yaml_content).map_err(|err| {
            CouldNotDecodeConfiguration(config_path.to_string(), err)
        })?;
        Ok(archive)
    }

    // Only for testing purposes
    pub fn write_to_file(&self, path: &std::path::Path) -> io::Result<()> {
        let serialized = serde_yaml::to_string(&self).map_err(|err| {
            Error::new(
                InvalidData,
                format!("Could not Serialize config, error: {}", err),
            )
        })?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(serialized.as_bytes())
    }
}

pub async fn handle_get_repository_config(
    State(shared_object): State<Arc<Repository>>,
) -> impl IntoResponse {
    Json(shared_object.config.clone())
}
