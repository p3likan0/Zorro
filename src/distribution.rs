use crate::database;
use crate::packages;
use crate::repository;
use serde::{Deserialize, Serialize};
use serde_json::json;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    response::Json,
};
use derive_more::Display;
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

#[derive(Debug, Deserialize, Serialize, Display, Clone)]
#[display(fmt = "DistributionKey: {}", self)]
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
//Maybe it is smarter to do not track "published" and "not published" distributions.
// assume all distributions are published

pub async fn handle_get_packages_in_distribution(
    State(shared_object): State<Arc<repository::Repository>>,
    Query(query): Query<DistributionKey>,
) -> impl IntoResponse {
    // Maybe is better to receive this a json instead of go with query
    let dist = DistributionKey {
        name: query.name,
        component: query.component,
        architecture: query.architecture,
    };
    match database::get_packages_in_distribution(&shared_object.db_conn, &dist) {
        Ok(packages) => (StatusCode::OK, Json(packages)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{}", err)})),
        )
            .into_response(),
    }
}
