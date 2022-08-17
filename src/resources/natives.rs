use std::fs::File;

use tracing::info;

use super::version::Library;
use crate::{error::Result, path_with_launcher};
//TODO: handle pre 1.19 natives
pub fn extract_natives(libraries: &[Library]) -> Result<()> {
    let natives = libraries
        .iter()
        .filter(|library| {
            if let Some(rules) = &library.rules {
                let rule = &rules[0];
                if let Some(os) = rule.os.as_ref() {
                    #[cfg(target_os = "windows")]
                    return os.name.as_ref().unwrap() == "windows";
                    #[cfg(target_os = "linux")]
                    return os.name.as_ref().unwrap() == "linux";
                } else {
                    return false;
                };
                
            }

            return false;
        })
        .map(|library| {
            path_with_launcher("libraries/") + &library.downloads.artifact.path
        });
    
    for native in natives {
        let file = File::open(native)?;
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let library = archive.file_names().filter(|file| file.ends_with(".so")).next().unwrap().to_owned();
        let _ = std::fs::create_dir(path_with_launcher("natives")); //ignoring
        let mut outfile = File::create(path_with_launcher("natives/") + library.split("/").last().unwrap())?;
        std::io::copy(&mut archive.by_name(&library).unwrap(), &mut outfile)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::resources::version::get_available_versions;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn gets_natives() {
        let versions = get_available_versions().await.unwrap();
        let latest = versions.latest_release();
        let details = latest.get_details().await.unwrap();
        super::extract_natives(&details.libraries).unwrap();
    }
}