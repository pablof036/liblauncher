use serde_json::error;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum MojangAuthError {
    #[error("account was migrated, use email")]
    UsernameMigrated,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("too many recent logins attempts")]
    RateLimited,
    #[error("password is not valid, it has to be at least 3 characters long")]
    BadPassword,
    #[error("account was migrated to Microsoft, use other login method")]
    AccountMigrated,
    #[error("unknown error")]
    Unknown
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("there is no profile associated with this account")]
    ProfileNotFound,
    #[error("bad access token")]
    BadAccessToken
}

#[derive(Debug)]
pub enum StartupRequirement {
    Account, Assets, Client, Libraries, Java
}

#[derive(Debug, Error)]
pub enum GameProfileError {
    #[error("requirement to start game not available")]
    RequirementFailed(StartupRequirement)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("error with Mojang authentication")]
    MojangAuthError(#[from] MojangAuthError),
    #[error("error trying to obtain profile")]
    ProfileError(#[from] ProfileError),
    #[error("unknown network error")]
    NetworkError(#[from] reqwest::Error),
    #[error("could not find needed java version")]
    JavaVersionNotFoundError,
    #[error("could not connect to embedded database")]
    DatabaseConnectionError(#[from] diesel::ConnectionError),
    #[error("database operation error")]
    DatabaseError(#[from] diesel::result::Error),
    #[error("error in file io")]
    FileIOError(#[from] std::io::Error),
    #[error("file already exists {0}")]
    FileExists(String),
    #[error("error ocurred at game startup or during execution")]
    GameProfileError(#[from] GameProfileError)
}