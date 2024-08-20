use crate::database;
use crate::packages;
use crate::repository;
use serde::{Deserialize, Serialize};
use serde_json::json;

use axum::{extract::State, http::StatusCode, response::IntoResponse, response::Json};
use std::sync::Arc;

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

pub async fn handle_get_published_distributions(
    State(shared_object): State<Arc<repository::Repository>>,
) -> impl IntoResponse {
    match database::get_published_distributions(&shared_object.db_conn) {
        Ok(distributions) => (StatusCode::OK, Json(distributions)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("{}", err)
            })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DistributionKey {
    pub name: String,
    pub component: String,
    pub architecture: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DistributionPublishPackage {
    pub package: packages::PackageKey,
    pub distribution: DistributionKey,
}

pub async fn handle_add_package_to_distribution(
    State(shared_object): State<Arc<repository::Repository>>,
    axum::extract::Json(dist_package): axum::extract::Json<DistributionPublishPackage>,
) -> impl IntoResponse {
    match database::insert_package_to_distribution(
        &shared_object.db_conn,
        &dist_package.package,
        &dist_package.distribution,
    ) {
        Ok(_) => (StatusCode::OK).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("{}", err)
            })),
        )
            .into_response(),
    }
}

pub async fn handle_publish_distribution(
    State(shared_object): State<Arc<repository::Repository>>,
    axum::extract::Json(dist): axum::extract::Json<DistributionKey>,
) {
    //query the database to get all the packages for the desired publication
    //generate the packages information for each one of the packages and write it to /dists/distribution/{correct place}
}
