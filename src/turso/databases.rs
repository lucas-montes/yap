use serde::{Deserialize, Serialize};

use super::client::{
    DatabaseInstancesPlatform, DatabasesPlatform, TursoClient, TursoError, DATABASES,
    DATABASE_INSTANCES, ORGANIZATIONS,
};

#[derive(Debug, Deserialize, Default)]
pub struct Database {
    #[serde(rename = "DbId")]
    pub db_id: String,
    #[serde(rename = "Hostname")]
    pub hostname: String,
    #[serde(rename = "Name")]
    pub name: String,
    pub group: String,
    #[serde(rename = "primaryRegion")]
    pub primary_region: String,
    pub regions: Vec<String>,
    #[serde(rename = "type")]
    pub type_: String,
    pub version: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListDatabases {
    pub databases: Vec<Database>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RetrievedDatabase {
    pub database: Database,
}

#[derive(Debug, Deserialize, Default)]
pub struct DeletededDatabase {
    pub database: String,
}

#[derive(Debug, Serialize)]
pub struct CreateDatabase<'a> {
    name: &'a str,
    group: &'a str,
}

impl<'a> CreateDatabase<'a> {
    fn new(name: &'a str, group: &'a str) -> Self {
        Self { name, group }
    }
}

/// Database Platform
impl TursoClient<DatabasesPlatform> {
    /// Method to list databases
    pub async fn list(&self, organization_name: &str) -> Result<ListDatabases, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{DATABASES}");
        self.get(&url).await
    }

    /// Method to retrieve databases
    pub async fn retrieve(
        &self,
        organization_name: &str,
        database_name: &str,
    ) -> Result<Database, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{DATABASES}/{database_name}");
        self.get(&url).await
    }

    /// Method to create databases
    pub async fn create(
        &self,
        organization_name: &str,
        name: &str,
        group: &str,
    ) -> Result<Database, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{DATABASES}");
        self.post(&url, &CreateDatabase::new(name, group)).await
    }

    /// Method to delete databases
    pub async fn delete(
        &self,
        organization_name: &str,
        database_name: &str,
    ) -> Result<String, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{DATABASES}/{database_name}");
        self.delete_(&url).await
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct DatabaseInstance {
    pub hostname: String,
    pub name: String,
    pub region: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub uuid: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListDatabaseInstance {
    pub instances: Vec<DatabaseInstance>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RetrieveDatabaseInstance {
    pub instance: DatabaseInstance,
}

/// Database Instance Platform
impl TursoClient<DatabaseInstancesPlatform> {
    /// Method to list database instances
    pub async fn list(
        &self,
        organization_name: &str,
        database_name: &str,
    ) -> Result<ListDatabaseInstance, TursoError> {
        let url = format!(
            "{ORGANIZATIONS}/{organization_name}/{DATABASES}/{database_name}/{DATABASE_INSTANCES}"
        );
        self.get(&url).await
    }

    /// Method to retrieve a specific database instance
    pub async fn retrieve(
        &self,
        organization_name: &str,
        database_name: &str,
        instance_name: &str,
    ) -> Result<RetrieveDatabaseInstance, TursoError> {
        let url =
            format!("{ORGANIZATIONS}/{organization_name}/{DATABASES}/{database_name}/{DATABASE_INSTANCES}/{instance_name}");
        self.get(&url).await
    }
}
