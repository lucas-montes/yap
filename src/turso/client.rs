use std::marker::PhantomData;

use menva::get_env;
use reqwest::{Client, ClientBuilder, Response};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, IntoStaticStr};

pub const BASE_PATH: &str = "https://api.turso.tech/v1/";
pub const ORGANIZATIONS: &str = "organizations";
pub const DATABASES: &str = "databases";
pub const DATABASE_INSTANCES: &str = "instances";
pub const MEMBERS: &str = "members";
pub const GROUPS: &str = "groups";
pub const LOCATIONS: &str = "locations";
pub const CREATE_TOKEN: &str = "auth/tokens";
pub const INVALIDATE_TOKEN: &str = "auth/rotate";

pub struct BasePlatform;
pub struct OrganizationsPlatform;
pub struct LocationsPlatform;
pub struct MembersPlatform;
pub struct DatabasesPlatform;
pub struct DatabaseInstancesPlatform;
pub struct DatabaseTokensPlatform;
pub struct GroupsPlatform;

#[derive(Display, EnumString, IntoStaticStr)]
pub enum Authorization {
    #[strum(serialize = "full-access")]
    FullAccess,
    #[strum(serialize = "read-only")]
    ReadOnly
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

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct TursoError {
    pub error: String,
}

impl TursoError {
    fn new(msg: &str) -> TursoError {
        TursoError {
            error: msg.to_string(),
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
    pub fn groups(self) -> TursoClient<GroupsPlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }
    pub fn locations(self) -> TursoClient<LocationsPlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }
    pub fn databases(self) -> TursoClient<DatabasesPlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }

    pub fn database_instances(self) -> TursoClient<DatabaseInstancesPlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }

    pub fn organizations(self) -> TursoClient<OrganizationsPlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }

    pub fn members(self) -> TursoClient<MembersPlatform> {
        TursoClient {
            token: self.token,
            client: self.client,
            platform: PhantomData,
        }
    }

    pub async fn get<T: for<'a> Deserialize<'a>>(&self, url: &str) -> Result<T, TursoError> {
        let url = if url.starts_with("https") {
            url.to_string()
        } else {
            format!("{BASE_PATH}{url}")
        };
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await
            .unwrap();
        //TODO: handle error
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
        //TODO: handle error
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
        //TODO: handle error
        self.serialize(response).await
    }

    async fn serialize<T: for<'a> Deserialize<'a>>(
        &self,
        response: Response,
    ) -> Result<T, TursoError> {
        //TODO: handle errors
        let result: TursoResult<T> = serde_json::from_str(&response.text().await.unwrap()).unwrap();
        match result {
            TursoResult::Ok(v) => Ok(v),
            TursoResult::Err(err) => Err(err),
        }
    }
}
