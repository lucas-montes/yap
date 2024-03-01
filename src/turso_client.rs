use std::marker::PhantomData;

use menva::{get_env, read_env_file};
use reqwest::{Client, ClientBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};

pub async fn run() -> i16 {
    read_env_file(".env");
    let client = TursoClient::new();
    let orgs_client = client.organizations();
    let orgs = orgs_client.list().await;
    println!("{orgs:?}");
    let dbs = orgs_client.databases().list("lucas-montes").await;
    println!("{dbs:?}");
    0
}

struct BasePlatform;
struct DatabasePlatform;
struct DatabaseInstancePlatform;
struct DatabaseTokenPlatform;
struct OrganizationPlatform;
struct MemberPlatform;

const BASE_PATH: &str = "https://api.turso.tech/v1/";
const ORGANIZATIONS: &str = "organizations";
const DATABASES: &str = "databases";
const DATABASE_INSTANCES: &str = "instances";
const MEMBERS: &str = "members";
const CREATE_TOKEN: &str = "auth/tokens";
const INVALIDATE_TOKEN: &str = "auth/rotate";

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Member {
    email: String,
    role: String,
    username: String,
}

/// https://api.turso.tech/v1/organizations/{organizationName}/members
#[derive(Debug, Deserialize, Serialize, Clone)]
struct Members {
    members: Vec<Member>,
}

#[derive(Debug, Deserialize, Default)]
struct Database {
    #[serde(rename = "DbId")]
    db_id: String,
    #[serde(rename = "Hostname")]
    hostname: String,
    #[serde(rename = "Name")]
    name: String,
    group: String,
    #[serde(rename = "primaryRegion")]
    primary_region: String,
    regions: Vec<String>,
    #[serde(rename = "type")]
    type_: String,
    version: String,
}
#[derive(Debug, Deserialize, Serialize, Default)]
struct TursoError {
    error: String,
}

impl TursoError {
    fn new(msg: &str) -> TursoError {
        TursoError {
            error: msg.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct ListDatabases {
    databases: Vec<Database>,
}

#[derive(Debug, Deserialize, Default)]
struct RetrieveDatabases {
    database: Database,
}

#[derive(Debug, Deserialize, Default)]
struct DeleteDatabases {
    database: String,
}

#[derive(Debug, Serialize)]
struct CreateDatabase<'a> {
    name: &'a str,
    group: &'a str,
}

impl<'a> CreateDatabase<'a> {
    fn new(name: &'a str, group: &'a str) -> Self {
        Self { name, group }
    }
}

/// Database Platform
impl TursoClient<DatabasePlatform> {
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
struct DatabaseInstance {
    hostname: String,
    name: String,
    region: String,
    #[serde(rename = "type")]
    type_: String,
    uuid: String,
}

#[derive(Debug, Deserialize, Default)]
struct ListDatabaseInstance {
    instances: Vec<DatabaseInstance>,
}

#[derive(Debug, Deserialize, Default)]
struct RetrieveDatabaseInstance {
    instance: DatabaseInstance,
}

/// Database Instance Platform
impl TursoClient<DatabaseInstancePlatform> {
    /// Method to list database instances
    pub async fn list(
        &self,
        organization_name: &str,
        database_name: &str,
    ) -> Result<Vec<DatabaseInstance>, TursoError> {
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
    ) -> Result<DatabaseInstance, TursoError> {
        let url =
            format!("{ORGANIZATIONS}/{organization_name}/{DATABASES}/{database_name}/{DATABASE_INSTANCES}/{instance_name}");
        self.get(&url).await
    }
}

/// https://docs.turso.tech/api-reference/organizations/list
#[derive(Debug, Deserialize, Serialize, Clone)]
struct Organisation {
    blocked_reads: bool,
    blocked_writes: bool,
    name: String,
    overages: bool,
    slug: String,
    #[serde(rename = "type")]
    type_: String,
}

/// Organization Platform
impl TursoClient<OrganizationPlatform> {
    /// Method to list organizations
    pub async fn list(&self) -> Result<Vec<Organisation>, TursoError> {
        self.get(ORGANIZATIONS).await
    }

    /// Method to retrieve a specific organization
    pub async fn retrieve(
        &self,
        organization_name: &str,
    ) -> Result<Option<Organisation>, TursoError> {
        Ok(self
            .list()
            .await?
            .iter()
            .find(|o| o.name.eq(organization_name))
            .cloned())
    }
}

/// Member Platform
impl TursoClient<MemberPlatform> {
    /// Method to list members
    pub async fn list(&self, name: &str) -> Result<Members, TursoError> {
        let url = format!("{ORGANIZATIONS}/{name}/{MEMBERS}");
        self.get(&url).await
    }

    /// Method to add a specific member
    /// TODO: finish
    pub async fn add(&self, name: &str, username: &str) -> Result<Member, TursoError> {
        let url = format!("{ORGANIZATIONS}/{name}/{MEMBERS}/{username}");
        self.get(&url).await
    }
}

pub struct TursoClient<Platform = BasePlatform> {
    token: String,
    client: Client,
    platform: PhantomData<Platform>,
}

impl TursoClient {
    pub fn new() -> Self {
        Self {
            token: get_env("TOKEN"),
            client: ClientBuilder::new().build().unwrap(),
            platform: PhantomData,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum TursoResult<T> {
    Ok(T),
    Err(TursoError),
}

impl<Platform> TursoClient<Platform> {
    pub fn databases(self) -> TursoClient<DatabasePlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }

    pub fn database_instances(self) -> TursoClient<DatabaseInstancePlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }

    pub fn organizations(self) -> TursoClient<OrganizationPlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }

    pub fn members(self) -> TursoClient<MemberPlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }

    pub async fn get<T: for<'a> Deserialize<'a>>(&self, url: &str) -> Result<T, TursoError> {
        let response = self
            .client
            .get(format!("{BASE_PATH}{url}"))
            .bearer_auth(&self.token)
            .send()
            .await
            .unwrap();
        self.serialize(response).await
    }

    pub async fn post<T: for<'a> Deserialize<'a>>(
        &self,
        url: &str,
        body: &impl Serialize,
    ) -> Result<T, TursoError> {
        let response = self
            .client
            .post(format!("{BASE_PATH}{url}"))
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
            .unwrap();
        self.serialize(response).await
    }

    pub async fn delete_<T: for<'a> Deserialize<'a>>(&self, url: &str) -> Result<T, TursoError> {
        let response = self
            .client
            .delete(format!("{BASE_PATH}{url}"))
            .bearer_auth(&self.token)
            .send()
            .await
            .unwrap();
        self.serialize(response).await
    }

    async fn serialize<T: for<'a> Deserialize<'a>>(
        &self,
        response: Response,
    ) -> Result<T, TursoError> {
        let result: TursoResult<T> = serde_json::from_str(&response.text().await.unwrap()).unwrap();
        match result {
            TursoResult::Ok(v) => Ok(v),
            TursoResult::Err(err) => Err(err),
        }
    }
}
