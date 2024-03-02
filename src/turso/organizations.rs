use serde::{Deserialize, Serialize};

use super::client::{
    MembersPlatform, OrganizationsPlatform, TursoClient, TursoError, MEMBERS, ORGANIZATIONS,
};

/// https://docs.turso.tech/api-reference/organizations/list
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Organisation {
    pub blocked_reads: bool,
    pub blocked_writes: bool,
    pub name: String,
    pub overages: bool,
    pub slug: String,
    #[serde(rename = "type")]
    pub type_: String,
}

/// Organization Platform
impl TursoClient<OrganizationsPlatform> {
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Member {
    pub email: String,
    pub role: String,
    pub username: String,
}

/// https://api.turso.tech/v1/organizations/{organizationName}/members
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Members {
    pub members: Vec<Member>,
}

/// Member Platform
impl TursoClient<MembersPlatform> {
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
