use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::io::Write;
use serde_json::json;

use axum::{
    response::Json,
    response::IntoResponse,
    http::StatusCode,
    extract::State,
};

use std::sync::Arc;
use std::{io, io::Error, io::ErrorKind::{InvalidData}};
use crate::database;

#[derive(Debug, Clone)]
pub struct Repository {
    pub config: RepositoryConfig,
    pub db_conn: database::Pool,
}

impl Repository {
    pub fn new(config_path: &str) -> io::Result<Repository> {
        let config = RepositoryConfig::new(&config_path)?;
        let db_conn = database::init_db_pool_connection(&config.db_file)?; 
        Ok(Repository{
            config,
            db_conn
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepositoryConfig {
    pub db_file: String,
    pub uploads_dir: String,
    pub pool_dir: String,
    pub dists: HashMap<String, Distribution>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Distribution {
    pub origin: String,
    pub label: String,
    pub version: String,
    pub codename: String,
    pub description: String,
    pub components: Vec<String>,
    pub architectures: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PublishedDistribution {
    pub name: String,
    pub origin: String,
    pub label: String,
    pub version: String,
    pub codename: String,
    pub description: String,
    pub component: String,
    pub architecture: String,
}


impl RepositoryConfig{
    pub fn new(config_path: &str) -> io::Result<RepositoryConfig> {
        let yaml_content = read_to_string(config_path)?;
        let archive: RepositoryConfig = serde_yaml::from_str(&yaml_content)
            .map_err(|err| {Error::new(InvalidData, format!("Could not decode yaml config: {}, error: {}", config_path, err))})?;
        Ok(archive)
    }

    // Only for testing purposes
    pub fn write_to_file(&self, path: &std::path::Path) -> io::Result<()> {
        let serialized = serde_yaml::to_string(&self)
            .map_err(|err| {Error::new(InvalidData, format!("Could not Serialize config, error: {}", err))})?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(serialized.as_bytes())
    }
}

pub async fn handle_get_repository_config(State(shared_object): State<Arc<Repository>>) -> impl IntoResponse {
    Json(shared_object.config.clone())
}

pub async fn handle_get_published_distributions(State(shared_object): State<Arc<Repository>>) -> impl IntoResponse {
    match database::get_published_distributions(&shared_object.db_conn){
        Ok(distributions) => (StatusCode::OK, Json(distributions)).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
            "error": format!("{}", err)
        }))).into_response()
    }
}
