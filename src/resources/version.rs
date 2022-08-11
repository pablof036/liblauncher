use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{client, error::Result};

use super::download::{self, Downloadeable, DownloadWithSizeCheck, DownloadType};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Argument {
    Simple(String),
    Detailed(PlatformArgument)
}

impl Argument {
    pub fn simple(&self) -> Option<&str> {
        if let Argument::Simple(arg) = self {
            Some(arg)
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
enum DetailedArgumentValue {
    Single(String),
    List(Vec<String>)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Versions {
    latest: Latest,
    versions: Vec<Version>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: VersionType,
    pub url: String,
    pub time: chrono::DateTime<chrono::Utc>,
    pub release_time: chrono::DateTime<chrono::Utc>,
}

impl Version {
    pub async fn get_details(&self) -> Result<VersionDetails> {
        Ok(client.get(&self.url).send().await?.json().await?)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetails {
    pub arguments: Arguments,
    pub asset_index: AssetIndex,
    pub assets: String,
    pub downloads: Downloads,
    pub java_version: JavaVersion,
    pub libraries: Vec<Library>,
    pub main_class: String,
    pub minimum_launcher_version: i32,
    pub release_time: chrono::DateTime<chrono::Utc>,
    pub time: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "type")]
    pub version_type: VersionType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Arguments {
    pub game: Vec<Argument>,
    pub jvm: Vec<Argument>
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformArgument {
    rules: Vec<PlatformRule>,
    value: DetailedArgumentValue
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformRule {
    pub action: String,
    pub os: Option<PlatformRuleOS>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformRuleOS {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub hash: String,
    pub size: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    id: String,
    sha1: String,
    size: i32,
    total_size: i32,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloads {
    client: Download,
    client_mappings: Option<Download>,
    server: Download,
    server_mappings: Option<Download>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    sha1: String,
    size: i32,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    component: String,
    major_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub name: String,
    pub downloads: LibraryDownload,
    pub rules: Option<Vec<PlatformRule>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDownload {
    pub artifact: LibraryArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryArtifact {
    pub path: String,
    pub sha1: String,
    pub size: i32,
    pub url: String,
}

pub async fn get_available_versions() -> Result<Versions> {
    Ok(client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest.json")
        .send()
        .await?
        .json()
        .await?)
}

impl Versions {
    pub fn latest_release(&self) -> &Version {
        self.versions
            .iter()
            .find(|v| v.id == self.latest.release)
            .unwrap()
    }

    pub fn latest_snapshot(&self) -> &Version {
        self.versions
            .iter()
            .find(|v| v.id == self.latest.snapshot)
            .unwrap()
    }

    pub fn all(&self) -> &Vec<Version> {
        &self.versions
    }

    pub fn release(&self) -> Vec<&Version> {
        self.filter_by_version_type(VersionType::Release)
    }

    pub fn snapshot(&self) -> Vec<&Version> {
        self.filter_by_version_type(VersionType::Snapshot)
    }

    pub fn old_beta(&self) -> Vec<&Version> {
        self.filter_by_version_type(VersionType::OldBeta)
    }

    pub fn old_alpha(&self) -> Vec<&Version> {
        self.filter_by_version_type(VersionType::OldAlpha)
    }

    fn filter_by_version_type(&self, version_type: VersionType) -> Vec<&Version> {
        self.versions
            .iter()
            .filter(|v| v.version_type == version_type)
            .collect()
    }
}

impl VersionDetails {
    fn index_path(&self) -> String {
        format!("./assets/indexes/{}.json", self.assets)
    }

    pub async fn get_assets(&self) -> Result<Vec<(String, Asset)>> {
        let index = tokio::fs::read_to_string(self.index_path()).await?;
        let assets = serde_json::from_str::<HashMap<String, HashMap<String, Asset>>>(&index).unwrap().remove("objects").unwrap();
        Ok(assets.into_iter().collect())
    }

    pub async fn store_asset_index(&self) -> Result<()> {
        let response = client
            .get(&self.asset_index.url)
            .send()
            .await?
            .text()
            .await?;
        
        
        tokio::fs::create_dir_all("./assets/indexes").await?;
        tokio::fs::write(self.index_path(), response).await?;
        Ok(())
    }

    pub fn client_download_info(&self) -> DownloadType {
        DownloadType::SizeCheck(
            DownloadWithSizeCheck {
                    download: download::Download {
                        path: format!("client/{}/client.jar", self.assets),
                        url: self.downloads.client.url.clone()
                    },
                    size: self.downloads.client.size as usize
            }
        )
    }
}

impl Downloadeable for (String, Asset) {
    fn download_info(&self) -> DownloadType {
        DownloadType::SizeCheck(DownloadWithSizeCheck {
            download: download::Download {
                path: format!("./assets/objects/{}/{}", &self.1.hash[..2], &self.1.hash),
                url: format!("http://resources.download.minecraft.net/{}/{}", &self.1.hash[..2], &self.1.hash)
            },
            size: self.1.size as usize
        })
    }
}


impl Downloadeable for Library {
    fn download_info(&self) -> DownloadType {
        DownloadType::SizeCheck(DownloadWithSizeCheck {
            download: download::Download {
                path: format!("./libraries/{}", self.downloads.artifact.path),
                url: self.downloads.artifact.url.clone()
            },
            size: self.downloads.artifact.size as usize
        })
    }
}

#[cfg(test)]
mod tests {
    use super::VersionType;
    use tracing::info;
    use tracing_test::traced_test;

    use super::get_available_versions;

    #[tokio::test]
    #[traced_test]
    async fn gets_versions() {
        let versions = get_available_versions().await.unwrap();
        info!("{:#?}", versions)
    }

    #[tokio::test]
    #[traced_test]
    async fn filters_by_release() {
        let versions = get_available_versions().await.unwrap();
        for version in versions.release() {
            info!("{:#?}", version);
            assert_eq!(version.version_type, VersionType::Release)
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn latest_release() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_release();
        info!("{:#?}", latest);
        assert_eq!(latest.id, versions.latest.release);
    }

    #[tokio::test]
    #[traced_test]
    async fn latest_snapshot() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_snapshot();
        info!("{:#?}", latest);
        assert_eq!(latest.id, versions.latest.snapshot);
    }

    #[tokio::test]
    #[traced_test]
    async fn get_details_for_latest_version() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_release();
        let details = latest.get_details().await.unwrap();
        info!("{:#?}", details);
    }

    #[tokio::test]
    #[traced_test]
    async fn get_assets_downloads_for_latest_version() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_release();
        let details = latest.get_details().await.unwrap();
        let assets = details.get_assets().await.unwrap();
        info!("{:#?}", assets);
    }
}
