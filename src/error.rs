use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    // #[error("Configuration 'config.yaml' introuvable. Exécutez 'init' pour commencer.")]
    // ConfigNotFound(#[source] std::io::Error),

    #[error("Erreur de lecture du fichier de configuration 'config.yaml'.")]
    ConfigReadError(#[source] std::io::Error),

    #[error("Le format de 'config.yaml' est invalide : {0}")]
    ConfigParseError(#[from] serde_yaml::Error),

    #[error("Le fichier source '{0}' spécifié dans 'config.yaml' est introuvable.")]
    SourceNotFound(String),

    #[error("Le projet existe déjà. 'init' ne peut être exécuté que dans un dossier non initialisé.")]
    ProjectAlreadyExists,

    #[error("Erreur d'entrée/sortie : {0}")]
    IoError(#[from] std::io::Error),

    #[error("Erreur lors de la construction de l'ebook : {0}")]
    BuildError(String),

    // #[error("Erreur lors de l'initialisation du projet : {0}")]
    // InitError(String),
}
