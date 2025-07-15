mod builder;
mod error;

use clap::{Parser, Subcommand};
use colored::*;
use serde::{Deserialize, Serialize};
use crate::error::AppError;

/// Un constructeur de PDF simple et rapide à partir de Markdown.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Construit l'ebook en se basant sur le fichier config.yaml.
    Build,
    /// Initialise un nouveau projet avec les fichiers de base.
    Init {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        language: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub title: String,
    pub author: String,
    pub language: String,
    pub theme: String,
    pub syntax_theme: String,
    pub source: String,
    pub custom_css: Option<String>,
    pub output: OutputConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputConfig {
    pub filename: String,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{} {}", "Erreur:".red().bold(), e.to_string().red());
        std::process::exit(1);
    }
}

async fn run() -> Result<(), AppError> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Build => {
            let config_str = std::fs::read_to_string("config.yaml").map_err(AppError::ConfigReadError)?;
            let config: Config = serde_yaml::from_str(&config_str)?;

            if !std::path::Path::new(&config.source).exists() {
                return Err(AppError::SourceNotFound(config.source.clone()));
            }

            builder::run_build(&config).await?;
        }
        Commands::Init { title, author, language } => {
            if std::path::Path::new("config.yaml").exists() {
                return Err(AppError::ProjectAlreadyExists);
            }
            builder::init_project(title.clone(), author.clone(), language.clone())?;
        }
    }

    Ok(())
}