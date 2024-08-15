use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::fs::read_to_string;

use axum::{
    response::Json,
    response::IntoResponse,
    extract::State,
};

use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepositoryConfig {
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

impl RepositoryConfig{
    pub fn new(config_path: &str) -> Self{
        let yaml_content = read_to_string(config_path)
            .unwrap_or_else(|_| panic!("Cannot read config: {}", config_path));
        println!("yaml: {}",yaml_content);
        let archive: RepositoryConfig = serde_yaml::from_str(&yaml_content)
            .unwrap_or_else(|_| panic!("Cannot parse yaml: {}", config_path));
        //let archive: Result<RepositoryConfig, serde_yaml::Error> = serde_yaml::from_str(&yaml_content);
        //match archive {
        //    Ok(value) => return value,
        //    Err(e) => panic!("Failed to parse as Value: {:?}", e),
        //}
        archive
    }

    pub fn print_structure(self){
        println!("{:#?}", self);
    }
}

pub async fn handle_get_repositories(
    State(shared_object): State<Arc<RepositoryConfig>>,
) -> impl IntoResponse {
    Json(shared_object.as_ref().clone())
}
