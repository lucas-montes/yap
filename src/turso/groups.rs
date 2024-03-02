use serde::{Deserialize, Serialize};

use super::client::{
    GroupsPlatform, TursoClient, TursoError, GROUPS, ORGANIZATIONS,
};

#[derive(Debug, Deserialize, Default)]
pub struct Group {
    pub archived: bool,
    pub locations: Vec<String>,
    pub name: String,
    pub primary: String,
    pub uuid: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListGroups {
    pub groups: Vec<Group>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RetrievedGroup {
    pub group: Group,
}

#[derive(Debug, Deserialize, Default)]
pub struct DeletededGroup {
    pub group: String,
}

#[derive(Debug, Serialize)]
pub struct CreateGroup<'a> {
    name: &'a str,
    location: &'a str,
}

impl<'a> CreateGroup<'a> {
    fn new(name: &'a str, location: &'a str) -> Self {
        Self { name, location }
    }
}

#[derive(Debug, Serialize)]
pub struct TransferGroup<'a> {
    organization: &'a str,
}

impl<'a> TransferGroup<'a> {
    fn new(organization: &'a str) -> Self {
        Self { organization }
    }
}

/// Group Platform
impl TursoClient<GroupsPlatform> {
    /// Method to transfer Groups
    pub async fn transfer(
        &self,
        organization_name: &str,
        name: &str,
        location: &str,
        group_name: &str,
    ) -> Result<Group, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{GROUPS}/{group_name}/transfer");
        self.post(&url, &TransferGroup::new(organization_name))
            .await
    }

    /// Method to list Groups
    pub async fn list(&self, organization_name: &str) -> Result<ListGroups, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{GROUPS}");
        self.get(&url).await
    }

    /// Method to retrieve Groups
    pub async fn retrieve(
        &self,
        organization_name: &str,
        group_name: &str,
    ) -> Result<RetrievedGroup, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{GROUPS}/{group_name}");
        self.get(&url).await
    }

    /// Method to create Groups
    pub async fn create(
        &self,
        organization_name: &str,
        name: &str,
        location: &str,
    ) -> Result<RetrievedGroup, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{GROUPS}");
        self.post(&url, &CreateGroup::new(name, location)).await
    }

    /// Method to delete Groups
    pub async fn delete(
        &self,
        organization_name: &str,
        group_name: &str,
    ) -> Result<RetrievedGroup, TursoError> {
        let url = format!("{ORGANIZATIONS}/{organization_name}/{GROUPS}/{group_name}");
        self.delete_(&url).await
    }
}
