use serde::{Deserialize, Serialize};

use super::client::{
    Authorization, DatabaseInstancesPlatform, DatabasesPlatform, TursoClient, TursoError,
    CREATE_TOKEN, DATABASES, DATABASE_INSTANCES, ORGANIZATIONS,
};

// After creating a new database the response give us some informations, thats why we use a default to
// avoid having an error when serializing the response
#[derive(Debug, Deserialize, Default)]
pub struct Database {
    #[serde(rename = "DbId")]
    pub db_id: String,
    #[serde(rename = "Hostname")]
    pub hostname: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(default)]
    pub group: String,
    #[serde(rename = "primaryRegion", default)]
    pub primary_region: String,
    #[serde(default)]
    pub regions: Vec<String>,
    #[serde(rename = "type", default)]
    pub type_: String,
    #[serde(default)]
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

#[derive(Debug, Serialize)]
pub struct CreateDatabaseToken<'a> {
    expiration: &'a str,
    authorization: &'a str,
}

impl<'a> CreateDatabaseToken<'a> {
    fn new(expiration: &'a str, authorization: &'a str) -> Self {
        Self {
            expiration,
            authorization,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct CreatedDatabaseToken {
    jwt: String,
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
    ) -> Result<RetrievedDatabase, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{DATABASES}/{database_name}");
        self.get(&url).await
    }

    /// Method to create databases
    pub async fn create(
        &self,
        organization_name: &str,
        name: &str,
        group: &str,
    ) -> Result<RetrievedDatabase, TursoError> {
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

    pub async fn create_token(
        &self,
        organization_name: &str,
        database_name: &str,
        authorization: &str,
        expiration: &str,
    ) -> Result<CreatedDatabaseToken, TursoError> {
        let url = format!(
            "{ORGANIZATIONS}/{organization_name}/{DATABASES}/{database_name}/{CREATE_TOKEN}"
        );
        self.post(&url, &CreateDatabaseToken::new(expiration, authorization))
            .await
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
