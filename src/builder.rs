use crate::error::AppError;
use crate::Config;
use axum::{routing::get_service, Router};
use colored::*;
use headless_chrome::{Browser, LaunchOptions};
use indicatif::ProgressBar;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use syntect::highlighting::ThemeSet;
use syntect::html::{css_for_theme_with_class_style, highlighted_html_for_string, ClassStyle};
use syntect::parsing::SyntaxSet;
use tower_http::services::ServeDir;

const DEFAULT_THEME_CSS: &str = r#"/* Simple Dark Theme */
body { background-color: #1a1a1a; color: #f2f2f2; font-family: sans-serif; line-height: 1.6; padding: 2em; }
h1, h2, h3 { color: #ffa500; }
code { background-color: #2a2a2a; padding: 2px 4px; border-radius: 4px; }
pre { background-color: #2a2a2a; padding: 1em; border-radius: 8px; overflow-x: auto; }
@media print {
    body { color: black; padding: 0; margin: 0; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
    .page-break { page-break-before: always !important; height: 0; overflow: hidden; line-height: 0; }
    h1 { page-break-before: always; }
    p, li { orphans: 3; widows: 3; }
    pre, code { background-color: #f0f0f0; border: 1px solid #ddd; page-break-inside: avoid; }
}"#;

pub async fn run_build(config: &Config) -> Result<(), AppError> {
    // Define the project root as the current working directory.
    // All file operations will be relative to this root.
    let project_root = std::env::current_dir()?;
    
    let full_markdown = preprocess_markdown(&project_root, &config.source, &mut HashSet::new())?;
    let assets_source_dir = PathBuf::from("assets");
    let assets_dest_dir = PathBuf::from("build").join("assets");
    copy_assets_optimized(&assets_source_dir, &assets_dest_dir)?;
    let (html_content, output_html_path) = build_html(config, &full_markdown)?;
    build_pdf_from_html(&html_content, &output_html_path).await?;

    println!("\n{}", "--------------------------------------------------".green());
    println!("{} ", "Build completed successfully!".green());
    println!("{} {}", "Generated HTML file:".cyan(), output_html_path.display().to_string().yellow());
    println!("{} {}", "Generated PDF file:".cyan(), output_html_path.with_extension("pdf").display().to_string().yellow());
    println!("{} ", "--------------------------------------------------".green());

    Ok(())
}

fn preprocess_markdown(project_root: &Path, file_path: &str, visited: &mut HashSet<String>) -> Result<String, AppError> {
    if !visited.insert(file_path.to_string()) {
        return Err(AppError::BuildError(format!("Circular dependency detected: '{file_path}'")));
    }

    #[cfg(not(test))]
    println!("{} {}", "Processing:".blue(), file_path.yellow());
    
    let content = fs::read_to_string(file_path).map_err(|_| AppError::SourceNotFound(file_path.to_string()))?;
    let include_re = Regex::new(r"^\s*!include\(([^)]+)\)\s*$").map_err(|e| AppError::BuildError(e.to_string()))?;

    let mut full_content = String::new();
    let mut in_code_block = false;

    for line in content.lines() {
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
        }

        if !in_code_block {
            if include_re.is_match(line) {
                if let Some(caps) = include_re.captures(line) {
                    let include_path_str = caps.get(1).unwrap().as_str();
                    
                    let base_path = Path::new(file_path).parent().unwrap_or_else(|| Path::new(""));
                    let include_path = base_path.join(include_path_str);
                    
                    // --- Security: Path Traversal Check ---
                    let canonical_path = path_clean::clean(include_path.to_str().unwrap());
                    let canonical_path = project_root.join(canonical_path);

                    if !canonical_path.starts_with(project_root) {
                        return Err(AppError::BuildError(format!("Unauthorized file access attempt: {}", include_path.display())));
                    }
                    // --- End of check ---
                    
                    let included_content = preprocess_markdown(project_root, include_path.to_str().unwrap_or(""), visited)?;
                    full_content.push_str(&included_content);
                    full_content.push('\n');
                }
            } else if line.trim() == "!newpage" {
                // Replace the directive with a div for the page break
                full_content.push_str("<div class=\"page-break\"></div>\n");
            } else {
                full_content.push_str(line);
                full_content.push('\n');
            }
        } else {
            full_content.push_str(line);
            full_content.push('\n');
        }
    }
    Ok(full_content)
}

fn build_html(config: &Config, markdown_content: &str) -> Result<(String, PathBuf), AppError> {
    #[cfg(not(test))]
    println!("{}", "Starting HTML build...".blue());
    let build_dir = Path::new("build");
    fs::create_dir_all(build_dir)?;

    let theme_css_path = Path::new("themes").join(&config.theme).join("style.css");
    let output_html_path = build_dir.join(format!("{}.html", config.output.filename));

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = ts.themes.get(&config.syntax_theme).ok_or_else(|| AppError::BuildError(format!("Syntax theme '{}' not found", config.syntax_theme)))?;

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown_content, options);
    let mut body_html = String::new();
    html::push_html(&mut body_html, parser);

    // Fix relative image paths
    let img_re = Regex::new(r#"<img src=\".\\./([^\"]+)\""#).map_err(|e| AppError::BuildError(e.to_string()))?;
    body_html = img_re.replace_all(&body_html, r#"<img src=\"$1\""#).to_string();

    let fragment = Html::parse_fragment(&body_html);
    let pre_selector = Selector::parse("pre").unwrap();
    let code_selector = Selector::parse("code[class*='language-']").unwrap();
    for pre_element in fragment.select(&pre_selector) {
        if let Some(code_element) = pre_element.select(&code_selector).next() {
            let lang = code_element.value().classes().find(|c| c.starts_with("language-")).map(|c| c.trim_start_matches("language-")).unwrap_or("text");
            let code = code_element.text().collect::<String>();
            let syntax = ss.find_syntax_by_token(lang).unwrap_or_else(|| ss.find_syntax_plain_text());
            let highlighted_code = highlighted_html_for_string(&code, &ss, syntax, theme).map_err(|e| AppError::BuildError(e.to_string()))?;
            body_html = body_html.replace(&pre_element.html(), &highlighted_code);
        }
    }

    let syntax_theme_css = css_for_theme_with_class_style(theme, ClassStyle::Spaced).map_err(|e| AppError::BuildError(e.to_string()))?;
    let theme_css = match fs::read_to_string(&theme_css_path) {
        Ok(s) => {
            #[cfg(not(test))]
            println!("{} {}", "Using custom CSS theme:".cyan(), theme_css_path.display().to_string().yellow());
            s
        }
        Err(_) => {
            #[cfg(not(test))]
            println!("{} {}{}", "Custom CSS theme not found at".yellow(), theme_css_path.display().to_string().yellow(), ". Using default theme.".yellow());
            DEFAULT_THEME_CSS.to_string()
        }
    };
    let mut final_css = format!("{}\n{}", theme_css, syntax_theme_css);

    if let Some(custom_css_path_str) = &config.custom_css {
        if !custom_css_path_str.is_empty() {
            match fs::read_to_string(custom_css_path_str) {
                Ok(s) => {
                    #[cfg(not(test))]
                    println!("{} {}", "Using custom CSS file:".cyan(), custom_css_path_str.yellow());
                    final_css.push_str("\n\n/* Custom CSS */\n");
                    final_css.push_str(&s);
                }
                Err(_) => {
                    #[cfg(not(test))]
                    println!("{} '{}' {}.", "Warning: Custom CSS file not found at".yellow(), custom_css_path_str.yellow(), "Ignored".yellow())
                },
            }
        }
    }

    let final_html = format!(r#"<!DOCTYPE html><html lang="{}"><head><meta charset="UTF-8"><title>{}</title><meta name="author" content="{}"><style>{}</style></head><body><main>{}</main></body></html>"# , config.language, config.title, config.author, final_css, body_html);
    fs::write(&output_html_path, &final_html)?;
    
    #[cfg(not(test))]
    println!("{} {}", "Standalone HTML generated:".green(), output_html_path.display().to_string().yellow());

    Ok((final_html, output_html_path))
}

async fn build_pdf_from_html(_html_content: &str, html_path: &Path) -> Result<(), AppError> {
    let pb = ProgressBar::new_spinner();
    pb.set_message(format!("{}", "Starting PDF conversion...".blue()));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let browser_path = find_browser_executable()?;
    let browser = Browser::new(LaunchOptions { path: Some(browser_path), ..Default::default() }).map_err(|e| AppError::BuildError(format!("Could not launch browser: {e}")))?;
    let tab = browser.new_tab().map_err(|e| AppError::BuildError(e.to_string()))?;

    let app = Router::new().nest_service("/", get_service(ServeDir::new("build")));
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_port = listener.local_addr()?.port();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service()).with_graceful_shutdown(async { shutdown_rx.await.ok(); }).await.unwrap();
    });

    let local_url = format!("http://127.0.0.1:{}/{}", actual_port, html_path.file_name().unwrap().to_str().unwrap());
    pb.set_message(format!("{} {}", "Navigating to:".blue(), local_url.yellow()));
    tab.navigate_to(&local_url).map_err(|e| AppError::BuildError(e.to_string()))?;
    tab.wait_for_element("body").map_err(|e| AppError::BuildError(e.to_string()))?;

    pb.set_message(format!("{}", "Generating PDF...".blue()));
    let pdf_path = html_path.with_extension("pdf");
    let pdf_data = tab.print_to_pdf(None).map_err(|e| AppError::BuildError(e.to_string()))?;
    fs::write(&pdf_path, pdf_data)?;
    pb.finish_with_message(format!("{} {}", "PDF generated: ".green(), pdf_path.display().to_string().yellow()));

    shutdown_tx.send(()).ok();
    server_task.await.map_err(|e| AppError::BuildError(e.to_string()))?;

    Ok(())
}

fn find_browser_executable() -> Result<PathBuf, AppError> {
    let candidates = [
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
        "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
    ];
    for path in candidates.iter() {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }
    Err(AppError::BuildError("No compatible browser (Chrome, Edge) was found.".to_string()))
}

pub fn init_project(title: Option<String>, author: Option<String>, language: Option<String>) -> Result<(), AppError> {
    #[cfg(not(test))]
    println!("{}", "Initializing a new project...".blue());

    let default_title = title.unwrap_or_else(|| "My Awesome PDF".to_string());
    let default_author = author.unwrap_or_else(|| "Your Name".to_string());
    let default_language = language.unwrap_or_else(|| "en".to_string());

    let config_content = format!(r#"title: "{}"
author: "{}"
language: "{}"
theme: "dark"
syntax_theme: "InspiredGitHub"
source: "main.md"
custom_css: ""
output:
  filename: "{}"
"#, default_title, default_author, default_language, default_title.to_lowercase().replace(" ", "-"));
    fs::write("config.yaml", config_content)?;
    #[cfg(not(test))]
    println!("{}", "'config.yaml' file created.".green());

    let main_md_content = format!(r#"# {}
By {}
Welcome!
!include(chapters/chapter1.md)
"#,
    default_title, default_author
    );
    fs::write("main.md", main_md_content)?;
    #[cfg(not(test))]
    println!("{}", "'main.md' file created.".green());

    fs::create_dir_all("chapters")?;
    fs::write("chapters/chapter1.md", "## Chapter 1\n\nContent of chapter 1.")?;
    #[cfg(not(test))]
    println!("{}", "'chapters/' directory and 'chapters/chapter1.md' created.".green());

    fs::create_dir_all("assets")?;
    #[cfg(not(test))]
    println!("{}", "'assets/' directory created.".green());

    #[cfg(not(test))]
    println!("\n{}", "Project initialized successfully!".green());
    #[cfg(not(test))]
    println!("{} {}", "To build, run:".cyan(), "PdfBuilder build".yellow());

    Ok(())
}

fn copy_assets_optimized(source_dir: &Path, dest_dir: &Path) -> Result<(), AppError> {
    if !source_dir.exists() {
        return Ok(()); 
    }
    fs::create_dir_all(dest_dir)?;
    #[cfg(not(test))]
    println!("{} {} {} {}...", "Copying assets from".blue(), source_dir.display().to_string().yellow(), "to".blue(), dest_dir.display().to_string().yellow());

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().ok_or_else(|| AppError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file name")))?;
        let dest_path = dest_dir.join(file_name);

        if path.is_file() {
            let should_copy = !dest_path.exists() || (fs::metadata(&path)?.modified()? > fs::metadata(&dest_path)?.modified()?);
            if should_copy {
                fs::copy(&path, &dest_path)?;
            }
        } else if path.is_dir() {
            copy_assets_optimized(&path, &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(test_name: &str) -> Self {
            let path = std::env::temp_dir().join("pdfbuilder_tests").join(test_name);
            if path.exists() {
                fs::remove_dir_all(&path).unwrap();
            }
            fs::create_dir_all(&path).unwrap();
            TestDir { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    #[test]
    fn test_init_project_creates_all_files() {
        let test_dir = TestDir::new("init_project_test");
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(test_dir.path()).unwrap();

        init_project(Some("Test Book".to_string()), None, None).unwrap();

        assert!(Path::new("config.yaml").exists());
        assert!(Path::new("main.md").exists());
        assert!(Path::new("chapters/chapter1.md").exists());
        assert!(Path::new("assets").is_dir());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_preprocess_markdown_simple() {
        let test_dir = TestDir::new("preprocess_simple");
        let file_path = test_dir.path().join("main.md");
        fs::write(&file_path, "Hello World").unwrap();

        let result = preprocess_markdown(test_dir.path(), file_path.to_str().unwrap(), &mut HashSet::new()).unwrap();
        assert_eq!(result.trim(), "Hello World");
    }

    #[test]
    fn test_preprocess_markdown_with_include() {
        let test_dir = TestDir::new("preprocess_include");
        let main_path = test_dir.path().join("main.md");
        let chap1_path = test_dir.path().join("chap1.md");

        fs::write(&main_path, "Book\n!include(chap1.md)").unwrap();
        fs::write(&chap1_path, "Content of chapter 1").unwrap();

        let result = preprocess_markdown(test_dir.path(), main_path.to_str().unwrap(), &mut HashSet::new()).unwrap();
        assert!(result.contains("Book"));
        assert!(result.contains("Content of chapter 1"));
    }

    #[test]
    fn test_preprocess_markdown_circular_dependency() {
        let test_dir = TestDir::new("preprocess_circular");
        let a_path = test_dir.path().join("a.md");
        let b_path = test_dir.path().join("b.md");

        fs::write(&a_path, "!include(b.md)").unwrap();
        fs::write(&b_path, "!include(a.md)").unwrap();

        let result = preprocess_markdown(test_dir.path(), a_path.to_str().unwrap(), &mut HashSet::new());
        assert!(matches!(result, Err(AppError::BuildError(_))));
    }

    #[test]
    fn test_preprocess_markdown_file_not_found() {
        let test_dir = TestDir::new("preprocess_not_found");
        let main_path = test_dir.path().join("main.md");
        fs::write(&main_path, "!include(nonexistent.md)").unwrap();

        let result = preprocess_markdown(test_dir.path(), main_path.to_str().unwrap(), &mut HashSet::new());
        assert!(matches!(result, Err(AppError::SourceNotFound(_))));
    }

    #[test]
    fn test_preprocess_markdown_path_traversal() {
        let test_dir = TestDir::new("preprocess_traversal");
        let project_dir = test_dir.path();
        
        // This file is outside the test project's "root", even if it's in the global test folder.
        let outside_file_path = std::env::temp_dir().join("secret.txt");
        fs::write(&outside_file_path, "secret content").unwrap();

        // The include path tries to go up the directory tree.
        let main_md_path = project_dir.join("main.md");
        // Use a relative path that would break out of the project root
        let traversal_path = if cfg!(windows) { "..\\..\\secret.txt" } else { "../../secret.txt" };
        fs::write(&main_md_path, format!("!include({})", traversal_path)).unwrap();

        let result = preprocess_markdown(project_dir, main_md_path.to_str().unwrap(), &mut HashSet::new());
        assert!(matches!(result, Err(AppError::BuildError(_))));
        
        // Cleanup the secret file
        fs::remove_file(outside_file_path).unwrap();
    }

    #[test]
    fn test_preprocess_markdown_ignores_include_in_code_block() {
        let test_dir = TestDir::new("preprocess_ignore_in_code");
        let main_path = test_dir.path().join("main.md");
        let content = "Example:\n```\n!include(some/file.md)\n```";
        fs::write(&main_path, content).unwrap();

        let result = preprocess_markdown(test_dir.path(), main_path.to_str().unwrap(), &mut HashSet::new()).unwrap();
        assert!(result.contains("!include(some/file.md)"));
    }

    #[test]
    fn test_preprocess_markdown_ignores_newpage_in_code_block() {
        let test_dir = TestDir::new("preprocess_ignore_newpage_in_code");
        let main_path = test_dir.path().join("main.md");
        let content = "Example:\n```\n!newpage\n```";
        fs::write(&main_path, content).unwrap();

        let result = preprocess_markdown(test_dir.path(), main_path.to_str().unwrap(), &mut HashSet::new()).unwrap();
        assert!(result.contains("!newpage"));
        assert!(!result.contains("<div class=\"page-break\"></div>"));
    }

    #[test]
    fn test_preprocess_markdown_handles_newpage() {
        let test_dir = TestDir::new("preprocess_newpage");
        let main_path = test_dir.path().join("main.md");
        let content = "Line 1\n!newpage\nLine 2";
        fs::write(&main_path, content).unwrap();

        let result = preprocess_markdown(test_dir.path(), main_path.to_str().unwrap(), &mut HashSet::new()).unwrap();
        assert!(result.contains("<div class=\"page-break\"></div>"));
        assert!(!result.contains("!newpage"));
    }
}