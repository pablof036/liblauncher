use std::collections::HashMap;

use glob::glob;
use serde::{Deserialize, Serialize};

use crate::path_with_launcher;
use crate::{client, error::Result};
use crate::resources::download::DownloadStatus;

use super::download::{self, Downloadeable, DownloadWithSizeCheck, DownloadType};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum DetailedArgumentValue {
    Single(String),
    List(Vec<String>)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Versions {
    latest: Latest,
    versions: Vec<Version>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetails {
    pub arguments: Arguments,
    asset_index: AssetIndex,
    pub assets: String,
    downloads: Downloads,
    pub java_version: JavaVersion,
    pub libraries: Vec<Library>,
    main_class: String,
    minimum_launcher_version: i32,
    release_time: chrono::DateTime<chrono::Utc>,
    time: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "type")]
    pub version_type: VersionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Arguments {
    pub game: Vec<Argument>,
    pub jvm: Vec<Argument>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformArgument {
    pub rules: Vec<PlatformRule>,
    pub value: DetailedArgumentValue
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub hash: String,
    pub size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    id: String,
    sha1: String,
    size: i32,
    total_size: i32,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloads {
    client: Download,
    client_mappings: Option<Download>,
    server: Download,
    server_mappings: Option<Download>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    sha1: String,
    size: i32,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: i32,
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
        path_with_launcher("assets/indexes/") + &self.assets + ".json"
    }

    pub async fn download_client(&self) -> Result<()>
    {
        self.client_download_info().download().await?;
        Ok(())
    }

    pub async fn download_jdk(&self) -> Result<()> {
        super::jdk::JavaVersion::search(self.java_version.major_version)
            .await?
            .download_info()
            .download()
            .await?;
        Ok(())
    }

    pub async fn download_assets<F>(&self, f: F) -> Result<()>
        where F: FnOnce(&download::Download, &Result<DownloadStatus>) + Clone {
        let assets = self.assets().await?;
        download::download_collection(&assets, |d, r| {
            download::log_download(d, r);
           f(d, r)
        }).await;
        Ok(())
    }

    pub async fn download_libraries<F>(&self, f: F) -> Result<()>
        where F: FnOnce(&download::Download, &Result<DownloadStatus>) + Clone  {
        download::download_collection(&self.libraries, |d, r| {
            download::log_download(d, r);
            f(d, r)
        }).await;
        Ok(())
    }

    pub fn check_jdk(&self) -> bool {
        glob(&(path_with_launcher("jdk/") + "*" + &self.java_version.major_version.to_string() +  "*/bin/java")).unwrap().count() != 0
    }

    pub async fn check_assets(&self) -> bool {
        super::download::is_downloaded(&self.assets().await.unwrap())
    }

    pub async fn check_libraries(&self) -> bool {
        super::download::is_downloaded(&self.libraries)
    }

    pub async fn check_client(&self) -> bool {
        self.client_download_info().size_check().unwrap().check_size()
    }

    pub fn extract_natives(&self) -> Result<()> {
        super::natives::extract_natives(&self.libraries)
    }

    async fn assets(&self) -> Result<Vec<(String, Asset)>> {
        return if let Ok(assets) = self.load_asset_index().await {
            Ok(assets)
        } else {
            self.store_asset_index().await?;
            self.load_asset_index().await
        }
    }

    async fn load_asset_index(&self) -> Result<Vec<(String, Asset)>> {
        let index = tokio::fs::read_to_string(self.index_path()).await?;
        let assets = serde_json::from_str::<HashMap<String, HashMap<String, Asset>>>(&index).unwrap().remove("objects").unwrap();
        Ok(assets.into_iter().collect())
    }

    async fn store_asset_index(&self) -> Result<()> {
        let response = client
            .get(&self.asset_index.url)
            .send()
            .await?
            .text()
            .await?;
        
        
        tokio::fs::create_dir_all(path_with_launcher("assets/indexes")).await?;
        tokio::fs::write(self.index_path(), response).await?;
        Ok(())
    }

    fn client_download_info(&self) -> DownloadType {
        DownloadType::SizeCheck(
            DownloadWithSizeCheck {
                    download: download::Download {
                        path: path_with_launcher("client/") + &self.assets + "/client.jar",
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
                path:  path_with_launcher("assets/objects/") + &self.1.hash[..2] + "/" + &self.1.hash,
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
                path: path_with_launcher("libraries/") +  &self.downloads.artifact.path,
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
    use crate::resources::download;

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
    async fn download_assets() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_release();
        let details = latest.get_details().await.unwrap();
        details.download_assets(|_, _| {}).await.unwrap();
    }
}
