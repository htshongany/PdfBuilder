use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Error reading the 'config.yaml' configuration file.")]
    ConfigReadError(#[source] std::io::Error),

    #[error("The format of 'config.yaml' is invalid: {0}")]
    ConfigParseError(#[from] serde_yaml::Error),

    #[error("The source file '{0}' specified in 'config.yaml' was not found.")]
    SourceNotFound(String),

    #[error("The project already exists. 'init' can only be run in an uninitialized directory.")]
    ProjectAlreadyExists,

    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Error while building the ebook: {0}")]
    BuildError(String),
}