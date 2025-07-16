mod builder;
mod error;

use crate::error::AppError;
use clap::{Parser, Subcommand};
use colored::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};


/// Un constructeur de PDF simple et rapide à partir de Markdown.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Construit l'ebook. Utilisez --watch pour recompiler automatiquement lors des changements.
    Build {
        /// Active le mode "watch" pour recompiler automatiquement lors des changements.
        #[arg(long)]
        watch: bool,
    },
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
        Commands::Build { watch } => {
            let config_str = std::fs::read_to_string("config.yaml").map_err(AppError::ConfigReadError)?;
            let config: Config = serde_yaml::from_str(&config_str)?;

            if !Path::new(&config.source).exists() {
                return Err(AppError::SourceNotFound(config.source.clone()));
            }

            // Première compilation
            builder::run_build(&config).await?;

            if *watch {
                println!("\n{}", "--------------------------------------------------".purple());
                println!("{}", "Mode Watch activé. En attente de changements...".purple());
                println!("{}", "Appuyez sur Ctrl+C pour quitter.".purple());
                println!("{}", "--------------------------------------------------".purple());
                run_watch_mode(config).await?;
            }
        }
        Commands::Init { title, author, language } => {
            if Path::new("config.yaml").exists() {
                return Err(AppError::ProjectAlreadyExists);
            }
            builder::init_project(title.clone(), author.clone(), language.clone())?;
        }
    }

    Ok(())
}

async fn run_watch_mode(config: Config) -> Result<(), AppError> {
    let (tx, rx) = channel();

    let watcher_config = NotifyConfig::default().with_poll_interval(Duration::from_secs(2));
    let mut watcher: RecommendedWatcher = Watcher::new(tx, watcher_config)
        .map_err(|e| AppError::BuildError(format!("Impossible de créer le watcher : {e}")))?;

    // Définir les chemins à surveiller
    watcher.watch(Path::new("config.yaml"), RecursiveMode::NonRecursive).unwrap();
    if let Some(parent) = Path::new(&config.source).parent() {
         if parent.to_str() != Some("") {
            watcher.watch(parent, RecursiveMode::Recursive).unwrap();
         } else {
            watcher.watch(Path::new("."), RecursiveMode::NonRecursive).unwrap();
         }
    }
    if let Some(css_path) = &config.custom_css {
        if !css_path.is_empty() && Path::new(css_path).exists() {
            watcher.watch(Path::new(css_path), RecursiveMode::NonRecursive).unwrap();
        }
    }
    if Path::new("assets").exists() {
        watcher.watch(Path::new("assets"), RecursiveMode::Recursive).unwrap();
    }
     if Path::new("themes").exists() {
        watcher.watch(Path::new("themes"), RecursiveMode::Recursive).unwrap();
    }


    for res in rx {
        match res {
            Ok(Event { kind, .. }) => {
                if kind.is_modify() || kind.is_create() || kind.is_remove() {
                    println!("\n{}", "--------------------------------------------------".cyan());
                    println!("{}", "Changement détecté, recompilation...".cyan());
                    if let Err(e) = builder::run_build(&config).await {
                        eprintln!("{} {}", "Erreur lors de la recompilation:".red().bold(), e.to_string().red());
                    }
                    println!("{}", "--------------------------------------------------".cyan());
                }
            }
            Err(e) => eprintln!("{} {}", "Erreur du watcher:".red().bold(), e.to_string().red()),
        }
    }

    Ok(())
}
