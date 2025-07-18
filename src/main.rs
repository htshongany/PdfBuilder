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


/// A simple and fast PDF builder from Markdown.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, short_flag = 'v')]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Builds the PDF. Use --watch to automatically recompile on changes.
    Build {
        /// Enables "watch" mode to automatically recompile on changes.
        #[arg(long)]
        watch: bool,
    },
    /// Initializes a new project with the base files.
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
        eprintln!("{} {}", "Error:".red().bold(), e.to_string().red());
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

            // First build
            builder::run_build(&config).await?;

            if *watch {
                println!("\n{}", "--------------------------------------------------".purple());
                println!("{}", "Watch mode enabled. Waiting for changes...".purple());
                println!("{}", "Press Ctrl+C to exit.".purple());
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
        .map_err(|e| AppError::BuildError(format!("Could not create watcher: {e}")))?;

    // Define paths to watch
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
                    clearscreen::clear().expect("failed to clear screen");
                    println!("{}", "--------------------------------------------------".cyan());
                    println!("{}", "Change detected, recompiling...".cyan());
                    println!("{}", "--------------------------------------------------".cyan());
                    if let Err(e) = builder::run_build(&config).await {
                        eprintln!("{} {}", "Error during recompilation:".red().bold(), e.to_string().red());
                    }
                }
            }
            Err(e) => eprintln!("{} {}", "Watcher error:".red().bold(), e.to_string().red()),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct TestProject {
        root: PathBuf,
    }

    impl TestProject {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join("pdfbuilder_cli_tests").join(name);
            if root.exists() {
                fs::remove_dir_all(&root).unwrap();
            }
            fs::create_dir_all(&root).unwrap();
            TestProject { root }
        }

        fn setup_build_success(&self) {
            let config_content = r#"
title: "Test Book"
author: "Test Author"
language: "en"
theme: "dark"
syntax_theme: "InspiredGitHub"
source: "main.md"
custom_css: ""
output:
  filename: "test-book"
"#;
            fs::write(self.root.join("config.yaml"), config_content).unwrap();
            fs::write(self.root.join("main.md"), "# Title\n\nText.").unwrap();
        }

        fn setup_build_failure(&self) {
            let config_content = r#"
title: "Test"
author: "Test"
language: "en"
theme: "dark"
syntax_theme: "InspiredGitHub"
source: "nonexistent.md"
custom_css: ""
output:
  filename: "test"
"#;
            fs::write(self.root.join("config.yaml"), config_content).unwrap();
        }
    }

    #[tokio::test]
    async fn test_cli_init_command() {
        let project = TestProject::new("cli_init");
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project.root).unwrap();

        let cli = Cli::parse_from(&["PdfBuilder", "init", "--title", "My Book"]);
        match &cli.command {
            Commands::Init { title, author, language } => {
                builder::init_project(title.clone(), author.clone(), language.clone()).unwrap();
            }
            _ => panic!("Wrong command"),
        }

        assert!(project.root.join("config.yaml").exists());
        let config_content = fs::read_to_string(project.root.join("config.yaml")).unwrap();
        assert!(config_content.contains("title: \"My Book\""));
        
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_cli_build_command_source_not_found() {
        let project = TestProject::new("cli_build_no_source");
        project.setup_build_failure();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project.root).unwrap();
        
        let config_str = std::fs::read_to_string("config.yaml").map_err(AppError::ConfigReadError).unwrap();
        let config: Config = serde_yaml::from_str(&config_str).unwrap();
        let build_result = if !Path::new(&config.source).exists() {
            Err(AppError::SourceNotFound(config.source.clone()))
        } else {
            Ok(())
        };

        assert!(matches!(build_result, Err(AppError::SourceNotFound(_))));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_cli_build_command_success() {
        let project = TestProject::new("cli_build_success");
        project.setup_build_success();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project.root).unwrap();

        let config_str = std::fs::read_to_string("config.yaml").map_err(AppError::ConfigReadError).unwrap();
        let config: Config = serde_yaml::from_str(&config_str).unwrap();
        
        let build_result = if !Path::new(&config.source).exists() {
            Err(AppError::SourceNotFound(config.source.clone()))
        } else {
            Ok(())
        };

        assert!(build_result.is_ok());

        std::env::set_current_dir(original_dir).unwrap();
    }
}
