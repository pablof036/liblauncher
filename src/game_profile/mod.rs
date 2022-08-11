use convert_case::Casing;
use tracing::info;

use crate::{
    error::{Error, GameProfileError, Result, StartupRequirement},
    resources::{
        download::{DownloadArchive, DownloadType, Downloadeable},
        version::{Argument, Arguments, Library, VersionDetails},
    },
};

pub struct Profile {
    name: String,
    version: String,
    java_path: String,
    argument_string: String,
    details: VersionDetails,
}

impl Profile {
    pub fn new(version: VersionDetails, name: &str, java_path: &str) -> Self {
        Self {
            name: name.to_owned(),
            version: version.assets.to_owned(),
            java_path: java_path.to_owned(),
            argument_string: Self::fill_arguments_str(
                &Self::join_arguments(&version.arguments),
                name,
                &version,
            ),
            details: version,
        }
    }

    pub async fn run(&self) -> Result<()> {
        self.check_requirements().await?;
        /* 
        tokio::process::Command::new(&self.java_path)
            .args(self.argument_string.split(" "))
            .spawn()?
            .wait()
            .await;
        Ok(())
        */
        todo!()
    }

    fn join_arguments(arguments: &Arguments) -> String {
        arguments
            .game
            .iter()
            .chain(arguments.jvm.iter())
            .filter(|arg| {
                return if let Argument::Simple(arg) = arg {
                    true
                } else {
                    false
                };
            })
            .fold(String::new(), |acc, arg| acc + " " + arg.simple().unwrap())
    }

    fn fill_arguments_str(arguments: &str, name: &str, details: &VersionDetails) -> String {
        let with_libraries = arguments.replace(
            "${classpath}",
            &(details
                .libraries
                .iter()
                .fold(String::new(), |acc, library| {
                    acc + &library.download_info().inner().path + ":"
                }) + "./client/" + &details.assets + "/client.jar"),
        );
        
        let with_natives = with_libraries.replace("${natives_directory}", "./libraries/");
        let with_assets_root = with_natives.replace("${assets_root}", "./assets");
        let with_assets_index = with_assets_root.replace("${assets_index_name}", &details.assets);

        let with_path = with_assets_index.replace(
            "${game_directory}",
            &format!("./instances/{}", name.to_case(convert_case::Case::Camel)),
        );
        let with_version = with_path.replace("${version_name}", &details.assets);
        let with_main_class = with_version + " " + "net.minecraft.client.main.Main";
        info!("{}", with_main_class);
        with_main_class
    }

    pub async fn check_requirements(&self) -> Result<()> {
        return if !is_downloaded(&self.details.libraries) {
            Err(Error::GameProfileError(
                GameProfileError::RequirementFailed(StartupRequirement::Libraries),
            ))
        } else if !is_downloaded(&self.details.get_assets().await?) {
            Err(Error::GameProfileError(
                GameProfileError::RequirementFailed(StartupRequirement::Assets),
            ))
        } else if !self
            .details
            .client_download_info()
            .size_check()
            .unwrap()
            .check_size()
        {
            Err(Error::GameProfileError(
                GameProfileError::RequirementFailed(StartupRequirement::Client),
            ))
        } else {
            Ok(())
        };
    }
}

pub fn is_downloaded(items: &[impl Downloadeable]) -> bool {
    for item in items.iter() {
        if !item.download_info().size_check().unwrap().check_size() {
            return false;
        }
    }

    return true;
}

#[cfg(test)]
mod tests {
    use crate::{game_profile::Profile, resources::download::DownloadeableCollection};
    use tracing::info;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn check_requirements() {
        let versions = crate::resources::version::get_available_versions()
            .await
            .unwrap();
        let latest = versions.latest_release().get_details().await.unwrap();

        let profile = Profile {
            name: String::from(""),
            argument_string: String::from(""),
            java_path: String::from(""),
            version: String::from(""),
            details: latest,
        };

        profile.check_requirements().await.unwrap();
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn arguments() {
        let versions = crate::resources::version::get_available_versions()
            .await
            .unwrap();
        let latest = versions.latest_release().get_details().await.unwrap();

        info!(
            "{}",
            Profile::fill_arguments_str(
                &Profile::join_arguments(&latest.arguments),
                "1.19",
                &latest
            )
        )
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn run() {
        let versions = crate::resources::version::get_available_versions()
            .await
            .unwrap();
        let latest = versions.release().iter().filter(|v| v.id == "1.14.4").next().unwrap().get_details().await.unwrap();
        //latest.client_download_info().download().await.unwrap();
        latest.libraries.download(|d, _| info!("{}", d.path)).await;
        latest.store_asset_index().await.unwrap();
        latest.get_assets().await.unwrap().download(|d, _| info!("{}", d.path)).await;
        let profile = Profile::new(latest, "1.14", "./jdk/8/jdk8u342-b07-jre/bin/java");

        profile.run().await.unwrap();
    }
}

//TODO: use glob for jdk
