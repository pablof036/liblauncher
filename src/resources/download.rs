//TODO: Fix, no like

use crate::error::Error;
use crate::{client, error::Result};

use async_trait::async_trait;
use flate2::bufread::GzDecoder;
use futures::StreamExt;
use tar::Archive;
use std::{fs, path::Path};
use tokio::time::Instant;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{error, info};

pub trait Downloadeable {
    fn download_info(&self) -> DownloadType;
}

pub enum DownloadType {
    Simple(Download),
    SizeCheck(DownloadWithSizeCheck),
    Archive(DownloadArchive)
}

impl DownloadType {
    pub async fn download(&self) -> Result<DownloadStatus> {
        match self {
            DownloadType::Simple(d) => d.download().await,
            DownloadType::SizeCheck(d) => d.download().await,
            DownloadType::Archive(d) => d.download().await,
        }
    }

    pub fn inner(&self) -> &Download {
        match self {
            DownloadType::Simple(d) => d,
            DownloadType::SizeCheck(d) => &d.download,
            DownloadType::Archive(d) => &d.download,
        }
    }

    pub fn size_check(self) -> Option<DownloadWithSizeCheck> {
        return if let Self::SizeCheck(size_check) = self {
            Some(size_check)
        } else {
            None
        }
    }
}

pub struct DownloadStatus {
    pub speed: f32,
    pub size: usize,
}

pub struct Download {
    pub path: String,
    pub url: String
}
pub struct DownloadWithSizeCheck {
    pub download: Download,
    pub size: usize
}

pub struct DownloadArchive {
    pub download: Download,
    pub destination: String,
}

pub fn is_downloaded(items: &[impl Downloadeable]) -> bool {
    for item in items.iter() {
        if !item.download_info().size_check().unwrap().check_size() {
            return false;
        }
    }

    return true;
}

pub fn log_download(item: &Download, result: &Result<DownloadStatus>) {
    match result {
        Ok(_) => info!("download success: {}", item.path),
        Err(e) => error!("download {} failed: {}", item.path, e),
    }
}

pub async fn download_collection<F>(collection: &[impl Downloadeable], f: F)
    where F: FnOnce(&Download, &Result<DownloadStatus>) + Clone {
    let tasks = tokio_stream::iter(collection)
        .map(|item| {
            let f = f.to_owned();
            let item = item.to_owned();
            async move {
                let download = item.download_info();
                let status = download.download().await;
                f(download.inner(), &status);
            }
        }).buffer_unordered(5);

    tasks.collect::<Vec<_>>().await;
}

impl Download {
    async fn download(&self) -> Result<DownloadStatus> {
        let mut body_stream = client.get(&self.url).send().await?.bytes_stream();
    
        if let Some(parent) = Path::new(&self.path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = File::create(&self.path).await?;
    
        let mut size = 0;
        let instant = Instant::now();
        while let Some(chunk) = body_stream.next().await {
            let chunk = chunk?;
            size += chunk.len();
            file.write_all(&chunk).await?;
        }
        let speed = size as f32 / instant.elapsed().as_secs_f32();
    
        Ok(DownloadStatus { speed, size })
    }
    
}

impl DownloadWithSizeCheck {
    pub fn check_size(&self) -> bool {
        let path_str = &self.download.path;
        let path = Path::new(path_str);

        return if path.exists() {
            let file_size = fs::File::open(path).unwrap().metadata().unwrap().len();
            file_size == self.size as u64
        } else {
            false
        };
    }

    async fn download(&self) -> Result<DownloadStatus> {
        if self.check_size() {
            return Err(Error::FileExists(self.download.path.to_owned()))
        }    
        
        Ok(self.download.download().await?)
    }
}

impl DownloadArchive {
    async fn download(&self) -> Result<DownloadStatus> {
        let status = self.download.download().await?;
        let file = tokio::fs::read(&self.download.path).await?;
        let destination = self.destination.clone();
        tokio::task::spawn_blocking(move || {
            
            let compressed = GzDecoder::new(file.as_slice());
            let mut archive = Archive::new(compressed);
            archive.unpack(&destination)
        }).await.unwrap()?;
        tokio::fs::remove_file(&self.download.path).await?;
        
        
        
        Ok(status)
    }
}



#[cfg(test)]
mod tests {
    use crate::{resources::version::get_available_versions, error};
    use super::{Download, DownloadStatus};
    use tracing::{info, error};
    use tracing_test::traced_test;



    #[tokio::test]
    #[traced_test]
    async fn download_assets() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_release();
        let details = latest.get_details().await.unwrap();
    }

    #[tokio::test]
    #[traced_test]
    async fn download_libraries() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_release();
        let details = latest.get_details().await.unwrap();

        
        super::download_collection(&details.libraries, |_, _| {}).await;
    }
}
