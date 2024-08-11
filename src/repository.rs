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
pub struct DebianArchive {
    dists: HashMap<String, HashMap<String, Section>>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Section {
    #[serde(flatten)]
    architectures: HashMap<String, Vec<String>>,
}

impl DebianArchive{
    pub fn new(config_path: &str) -> Self{
        let yaml_content = read_to_string(config_path)
            .unwrap_or_else(|_| panic!("Cannot read config: {}", config_path));
        let archive: DebianArchive = serde_yaml::from_str(&yaml_content)
            .unwrap_or_else(|_| panic!("Cannot parse yaml: {}", config_path));
        archive
    }

    pub fn print_structure(self){
        println!("{:#?}", self);
    }
}

pub async fn handle_get_repositories(
    State(shared_object): State<Arc<DebianArchive>>,
) -> impl IntoResponse {
    Json(shared_object.as_ref().clone())
}
