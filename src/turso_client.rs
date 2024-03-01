use std::marker::PhantomData;

use menva::{get_env, read_env_file};
use reqwest::{Client, ClientBuilder, Response};
use serde::{Deserialize, Serialize};

pub async fn run() -> i16 {
    read_env_file(".env");
    let client = TursoClient::new();
    0
}

struct DatabaseMode;
struct OrganizationMode;

pub struct TursoClient<Mode> {
    token: String,
    client: Client,
    state: PhantomData<Mode>
}

impl<Mode> TursoClient<Mode> {
    pub fn new() -> Self {
        Self {
            token: get_env("TOKEN"),
            client: ClientBuilder::new().build().unwrap(),
        }
    }
    pub async fn get(&self, url: &str) -> Result<Response, reqwest::Error> {
        self.client
            .get(format!("https://api.turso.tech/v1/{url}"))
            .bearer_auth(&self.token)
            .send()
            .await
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

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Member {
    email: String,
    role: String,
    username: String,
}

/// https://api.turso.tech/v1/organizations/{organizationName}/members
#[derive(Debug, Deserialize, Serialize, Clone)]
struct OrganisationMembers {
    members: Vec<Member>,
}

impl Organisation {
    async fn get_members(name: &str) -> OrganisationMembers {
        //let orga = Self::get(name).await.unwrap().name;
        let orga = "default";
        let response = Self::request(&format!("organizations/{orga}/members")).await;
        match response.json().await {
            Ok(r) => r,
            Err(err) => panic!("{err:?}"),
        }
    }
    async fn get(name: &str) -> Option<Self> {
        Self::get_all()
            .await
            .iter()
            .find(|o| o.name.eq(name))
            .cloned()
    }
    async fn request(client: &TursoClient, url: &str) -> Response {
        match client
            .get(url)
            .await
        {
            Ok(r) => r,
            Err(err) => panic!("{err:?}"),
        }
    }
    async fn get_all() -> Vec<Self> {
        let response = Self::request("organizations").await;
        match response.json().await {
            Ok(r) => r,
            Err(err) => panic!("{err:?}"),
        }
    }
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct ApiResponse {
    databases: Vec<Database>,
}
