use std::{iter::{Map, Filter}, path::PathBuf};

use convert_case::Casing;
use tracing::info;
use glob::glob;

use crate::{
    error::{Error, GameProfileError, Result, StartupRequirement},
    resources::version::{Argument, Arguments, Library, VersionDetails}, store::models::Account, path_with_launcher,
};

pub struct Profile {
    name: String,
    version: String,
    arguments: Vec<String>,
    details: VersionDetails,
}

impl Profile {
    pub fn new(version: &VersionDetails, name: &str) -> Self {
        Self {
            name: name.to_owned(),
            version: version.assets.to_owned(),
            arguments: Self::fill_static_arguments(&mut Self::parse_arguments(version), version, name) ,
            details: version.to_owned(),
        }
    }

    //TODO: dynamic java path resolution
    pub async fn run(&self, account: &Account) -> Result<()> {
        self.check_requirements().await?;
        
        let _ = tokio::process::Command::new(self.java_path())
            .args(self.fill_dynamic_args(account))
            .spawn()?
            .wait()
            .await;
        
        tokio::fs::remove_dir_all(path_with_launcher("natives")).await?;

        Ok(())
    }

    fn java_path(&self) -> PathBuf {
        //don't like copying
        glob(&(path_with_launcher("jdk/") + "*" + &self.details.java_version.major_version.to_string() +  "*/bin/java"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
    }

    fn parse_arguments(details: &VersionDetails) -> Vec<String> {
        let mut jvm_args = Self::filter_args(&details.arguments.jvm)
            .iter()
            .map(|arg| {
                let arg = arg.simple().unwrap();
                arg.to_owned()
            })
            .collect::<Vec<_>>();

        let mut game_args = Self::filter_args(&details.arguments.game)
            .iter()
            .map(|arg| {
                let arg = arg.simple().unwrap();
                arg.to_owned()
            })
            .collect::<Vec<_>>();

        jvm_args.push(String::from("net.minecraft.client.main.Main"));
        
        jvm_args.append(&mut game_args);

        jvm_args
    }

    fn filter_args(arguments: &[Argument]) -> Vec<&Argument>{
        arguments.iter().filter(|arg| 
           match arg {
            Argument::Simple(arg) => true,
            //TODO: parse this
            Argument::Detailed(arg) => false,
           }
        ).collect()
    }

    fn fill_static_arguments(arguments: &[String], details: &VersionDetails, name: &str) -> Vec<String> {
        //aaa
        arguments
            .iter()
            .map(|arg| match arg.as_ref() {
                "-Djava.library.path=${natives_directory}" => format!("-Djava.library.path=${}", path_with_launcher("natives")),
                "${classpath}" => (details
                    .libraries
                    .iter()
                    .fold(String::new(), |acc, library| {
                        acc + &path_with_launcher("libraries/") + &library.downloads.artifact.path + ":"
                    }) + &path_with_launcher("client/") + &details.assets + "/client.jar"),
                    "${assets_root}" => path_with_launcher("assets"),
                "${assets_index_name}" => details.assets.clone(),
                "${version_name}" => details.assets.clone(),
                "${game_directory}" => path_with_launcher("instances/") + &name.to_case(convert_case::Case::Camel),
                "${user_type}" => String::from("mojang"),
                "${version_type}" => format!("{:?}", details.version_type),
                _ => arg.clone()    
            })
            .collect()
    }
    
    fn fill_dynamic_args(&self, account: &Account) -> Vec<String> {
        self.arguments
            .iter()
            .map(|arg| match arg.as_ref() {
                "${auth_player_name}" => account.username.clone(),
                "${auth_uuid}" => account.account_uuid.clone(),
                "${auth_access_token}" => account.access_token.clone(),
                "${client_id}" => account.client_id.clone(),
                _ => arg.clone()
            })
            .collect()
    }

    //TODO: maybe check for asset index
    pub async fn check_requirements(&self) -> Result<()> {
        return if !self.details.check_libraries().await {
            Err(Error::GameProfileError(
                GameProfileError::RequirementFailed(StartupRequirement::Libraries),
            ))
        } else if !self.details.check_assets().await {
            Err(Error::GameProfileError(
                GameProfileError::RequirementFailed(StartupRequirement::Assets),
            ))
        } else if !self.details.check_client().await {
            Err(Error::GameProfileError(
                GameProfileError::RequirementFailed(StartupRequirement::Client),
            ))
        } else if !self.details.check_jdk() {
            Err(Error::GameProfileError(
                GameProfileError::RequirementFailed(StartupRequirement::Java),
            ))
        } else {
            Ok(())
        };
    }
}


#[cfg(test)]
mod tests {
    use crate::game_profile::Profile;
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
            arguments: vec![],
            version: String::from(""),
            details: latest,
        };

        profile.check_requirements().await.unwrap();
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn run() {
        let versions = crate::resources::version::get_available_versions()
            .await
            .unwrap();
        let latest = versions.latest_release().get_details().await.unwrap();
        //latest.download_client().await.unwrap();
        latest.download_libraries(|_, _| {}).await.unwrap();
        latest.download_assets(|_, _| {}).await.unwrap();
        latest.extract_natives().unwrap();
        
        let profile = Profile::new(&latest, "1.19");
        info!("{:#?}", profile.arguments);
        profile.run(&crate::store::models::Account { id: None, 
            client_id: String::from("liblauncher"), 
            access_token: String::new(), 
            account_uuid: String::new(), 
            username: String::from("Username") }
        ).await.unwrap();
    }
}

//TODO: use glob for jdk
