use crate::error::Error;
use crate::{client, error::Result};

use async_trait::async_trait;
use futures::StreamExt;
use std::{fs, path::Path};
use tokio::time::Instant;
use tokio::{fs::File, io::AsyncWriteExt};

pub enum DownloadResult {
    Success(SingleDownloadStatus),
    FileExists,
}

pub struct DownloadStatus {
    pub progress: f32,
    pub downloaded: i32,
    pub total: usize,
    pub speed: f32,
    pub last_size: usize,
    pub total_size: usize,
}

pub struct SingleDownloadStatus {
    pub speed: f32,
    pub size: usize,
}

async fn download<T: Downloadeable + ?Sized>(item: &T) -> Result<(&T, DownloadResult)> {
    let mut body_stream = client.get(&item.url()).send().await?.bytes_stream();

    if let Some(parent) = Path::new(&item.path()).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = File::create(item.path()).await?;

    let mut size = 0;
    let instant = Instant::now();
    while let Some(chunk) = body_stream.next().await {
        let chunk = chunk?;
        size += chunk.len();
        file.write_all(&chunk).await?;
    }
    let speed = size as f32 / instant.elapsed().as_secs_f32();

    Ok((
        item,
        DownloadResult::Success(SingleDownloadStatus { speed, size }),
    ))
}

async fn download_single<T: Downloadeable + ?Sized>(item: &T) -> Result<DownloadResult> {
    let (_, status) = download(item).await?;
    Ok(status)
}

#[async_trait]
pub trait Downloadeable {
    fn path(&self) -> String;
    fn url(&self) -> String;
    async fn download(&self) -> Result<DownloadResult> {
        download_single(self).await
    }
}

#[async_trait]
pub trait DownloadeableSizeCheck {
    fn path(&self) -> String;
    fn url(&self) -> String;
    fn size(&self) -> usize;
    fn check_size(&self) -> bool {
        let path_str = &self.path();
        let path = Path::new(path_str);

        return if path.exists() {
            let file_size = fs::File::open(path).unwrap().metadata().unwrap().len();
            file_size == self.size() as u64
        } else {
            false
        };
    }
}

pub trait DownloadeableArchive {
    fn path(&self) -> String;
    fn url(&self) -> String;
}

#[async_trait]
impl<T: DownloadeableSizeCheck + Sync> Downloadeable for T {
    fn path(&self) -> String {
        self.path()
    }

    fn url(&self) -> String {
        self.url()
    }

    async fn download(&self) -> Result<DownloadResult> {
        if self.check_size() {
            return Ok(DownloadResult::FileExists);
        }

        download_single(self).await
    }
}

#[async_trait]
trait DownloadeableCollection {
    type Item;
    async fn download<F>(self, progress: F) -> Result<()>
    where
        F: Fn(&DownloadStatus, Self::Item, DownloadResult) + Send;
}

//TODO: Downloadeable impl for single element

#[async_trait]
impl<T: Downloadeable + Sync + Send> DownloadeableCollection for Vec<T> {
    type Item = T;

    async fn download<F>(self, progress: F) -> Result<()>
    where
        F: Fn(&DownloadStatus, Self::Item, DownloadResult) + Send,
    {
        let len = self.len();

        let mut downloads = tokio_stream::iter(self)
            .map(|item| async move {
                let result = item.download().await?;
                Ok::<(T, DownloadResult), Error>((item, result))
            })
            .buffer_unordered(5);

        let mut status = DownloadStatus {
            progress: 0.0,
            downloaded: 0,
            total: len,
            speed: 0.0,
            last_size: 0,
            total_size: 0,
        };

        while let Some(item) = downloads.next().await {
            let (item, result) = item?;

            if let DownloadResult::Success(s_status) = &result {
                status.last_size = s_status.size;
                status.total_size += status.last_size;
                status.speed = s_status.speed;
            }

            status.downloaded += 1;
            status.progress = status.downloaded as f32 / status.total as f32;

            progress(&status, item, result);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::resources::{download::DownloadeableCollection, version::get_available_versions};
    use tracing::info;
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn download_assets() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_release();
        let details = latest.get_details().await.unwrap();
        let assets = details.get_assets().await.unwrap();

        assets
            .download(|status, (name, asset), result| {
                match result {
                    crate::resources::download::DownloadResult::Success(_) => {
                        info!(
                            "download: {}\n{:.2}%. Size: {:.2}KiB. Speed: {:.2}KiB/s.",
                            name,
                            status.progress * 100.0,
                            status.last_size as f32 / 1024.0,
                            status.speed / 1024.0
                        );
                    },
                    crate::resources::download::DownloadResult::FileExists => {
                        info!(
                            "file exists: {}",
                            name
                        )
                    },
                }

                
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    #[traced_test]
    async fn download_libraries() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_release();
        let details = latest.get_details().await.unwrap();

        details
            .libraries
            .download(|status, library, result| {
                info!("download: {}", library.name);
            })
            .await
            .unwrap();
    }
}
