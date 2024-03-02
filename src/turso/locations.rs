use std::collections::HashMap;

use serde::Deserialize;

use super::client::{LocationsPlatform, TursoClient, TursoError, LOCATIONS};

pub const REGIONS_PATH: &str = "https://region.turso.tech";

#[derive(Debug, Deserialize, Default)]
pub struct ClosetRegion {
    pub server: String,
    pub client: String,
}
#[derive(Debug, Deserialize, Default)]
pub struct Locations {
    pub locations: Vec<HashMap<String, String>>,
}

/// Locations Platform
impl TursoClient<LocationsPlatform> {
    /// Method to list all the locations
    pub async fn closet_region(&self) -> Result<ClosetRegion, TursoError> {
        self.get(REGIONS_PATH).await
    }
    /// Method to list all the locations
    pub async fn list(&self) -> Result<Locations, TursoError> {
        self.get(LOCATIONS).await
    }
}
