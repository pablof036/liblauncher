use serde::{Serialize, Deserialize};


use crate::{error::{Result, Error}, client};

use super::download::{Downloadeable, DownloadType, DownloadArchive, Download};

#[derive(Debug, Clone)]
pub struct JavaVersion {
    major_version: u32,
    download_url: String,
    filename: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoApiResult {
    result: Vec<DiscoApiPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoApiPackage {
    major_version: i32,
    filename: String,
    links: Links
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Links {
    pkg_download_redirect: String
}

impl JavaVersion {
    pub async fn search(major_version: i32) -> Result<Self> {
        let versions = client.get("https://api.foojay.io/disco/v3.0/packages")
            .query(&[
                #[cfg(target_arch="x86")]
                ("arch", "x86"),
                #[cfg(target_arch="x86_64")]
                ("architecture", "x86-64"),
                #[cfg(target_arch="arm")]
                ("architecture", "arm"),
                #[cfg(target_arch="aarch64")]
                ("architecture", "arm64"),
                #[cfg(target_arch="mips")]
                ("arch", "mips"),
                #[cfg(target_arch="powerpc")]
                ("architecture", "ppc"),
                #[cfg(target_arch="powerpc64")]
                ("architecture", "ppc64"),
                #[cfg(target_os = "windows")]
                ("operating_system", "windows"),
                #[cfg(target_os = "linux")]
                ("operating_system", "linux"),
                //#[cfg(target_os = "macos")]
                //("operating_system", "macos"),
                #[cfg(target_os = "windows")]
                ("archive_type", "zip"),
                #[cfg(target_os = "linux")]
                ("archive_type", "tar.gz"),
                ("package_type", "jre"),
                ("javafx_bundled", "false"),
                ("directly_downloadeable", "true"),
                ("free_use_in_production", "true"),
                ("distribution", "temurin,microsoft")
            ])
            .send().await?
            .json::<DiscoApiResult>().await?
            .result;
        
        let version = 
            versions
            .into_iter()
            .filter(|version| 
                version.major_version == major_version
                //even tough operating_system is set to "linux", the api returns "alpine-linux" versions, which are not compatible
                && !version.filename.contains("alpine-linux")
            )
            .next();
            
        return if let Some(version) = version {
            Ok(
                Self {
                    major_version: version.major_version as u32,
                    filename: version.filename,
                    download_url: version.links.pkg_download_redirect
                }
            )
        } else {
            Err(Error::JavaVersionNotFoundError)
        } 
    }
}

impl Downloadeable for JavaVersion {
    fn download_info(&self) -> DownloadType {
        DownloadType::Archive(
            DownloadArchive {
                download: Download {
                    path: format!("jdk/{}", self.filename),
                    url: self.download_url.clone()
                },
                destination: format!("jdk/{}", self.major_version)
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;
    use crate::resources::download::Downloadeable;

    use super::JavaVersion;
    use tracing::info;

    #[tokio::test]
    #[traced_test]
    async fn get_eleven() {
        let version = JavaVersion::search(8).await.unwrap();
        info!("{:#?}", version);
        version.download_info().download().await.unwrap();
    }
}