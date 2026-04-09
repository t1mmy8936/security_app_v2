use crate::db::{self, DbPool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AzureProject {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AzureRepo {
    pub id: String,
    pub name: String,
    pub default_branch: Option<String>,
    pub remote_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AzureBranch {
    pub name: String,
    pub object_id: String,
}

pub struct AzureDevOpsClient {
    org: String,
    pat: String,
    project: String,
    client: reqwest::Client,
}

impl AzureDevOpsClient {
    pub async fn from_settings(pool: &DbPool) -> Option<Self> {
        let org = db::get_setting(pool, "azure_devops_org").await;
        let pat = db::get_setting(pool, "azure_devops_pat").await;
        let project = db::get_setting(pool, "azure_devops_project").await;

        if org.is_empty() || pat.is_empty() {
            return None;
        }

        Some(Self {
            org,
            pat,
            project,
            client: reqwest::Client::new(),
        })
    }

    fn base_url(&self) -> String {
        format!("https://dev.azure.com/{}", self.org)
    }

    pub async fn list_projects(&self) -> Result<Vec<AzureProject>, String> {
        let url = format!("{}/_apis/projects?api-version=7.0", self.base_url());
        let resp = self.client.get(&url)
            .basic_auth("", Some(&self.pat))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut projects = Vec::new();

        if let Some(items) = body["value"].as_array() {
            for item in items {
                projects.push(AzureProject {
                    id: item["id"].as_str().unwrap_or("").into(),
                    name: item["name"].as_str().unwrap_or("").into(),
                });
            }
        }

        Ok(projects)
    }

    pub async fn list_repos(&self, project: &str) -> Result<Vec<AzureRepo>, String> {
        let url = format!("{}/{}/_apis/git/repositories?api-version=7.0", self.base_url(), project);
        let resp = self.client.get(&url)
            .basic_auth("", Some(&self.pat))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut repos = Vec::new();

        if let Some(items) = body["value"].as_array() {
            for item in items {
                repos.push(AzureRepo {
                    id: item["id"].as_str().unwrap_or("").into(),
                    name: item["name"].as_str().unwrap_or("").into(),
                    default_branch: item["defaultBranch"].as_str().map(|s| s.to_string()),
                    remote_url: item["remoteUrl"].as_str().map(|s| s.to_string()),
                });
            }
        }

        Ok(repos)
    }

    pub async fn list_branches(&self, project: &str, repo: &str) -> Result<Vec<AzureBranch>, String> {
        let url = format!("{}/{}/_apis/git/repositories/{}/refs?filter=heads/&api-version=7.0",
            self.base_url(), project, repo);
        let resp = self.client.get(&url)
            .basic_auth("", Some(&self.pat))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut branches = Vec::new();

        if let Some(items) = body["value"].as_array() {
            for item in items {
                let name = item["name"].as_str().unwrap_or("")
                    .trim_start_matches("refs/heads/")
                    .to_string();
                branches.push(AzureBranch {
                    name,
                    object_id: item["objectId"].as_str().unwrap_or("").into(),
                });
            }
        }

        Ok(branches)
    }

    pub async fn clone_repo(&self, remote_url: &str, dest: &str) -> Result<String, String> {
        // Build authenticated URL
        let auth_url = remote_url.replacen("https://", &format!("https://pat:{}@", self.pat), 1);

        let output = tokio::process::Command::new("git")
            .args(["clone", "--depth", "1", &auth_url, dest])
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(dest.to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}
