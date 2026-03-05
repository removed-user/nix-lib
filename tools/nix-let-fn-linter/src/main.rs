use clap::{Parser, ValueEnum};
use colored::Colorize;
use glob::Pattern;
use rnix::ast::HasEntry;
use rnix::{ast, SyntaxKind, SyntaxNode};
use rowan::ast::AstNode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Default skip directives that can appear in comments
const DEFAULT_SKIP_DIRECTIVES: &[&str] = &[
    "nix-let-fn-linter:disable",
    "nix-let-fn-linter: disable",
    "noqa: let-fn",
    "noqa:let-fn",
];

/// Config file names to search for (in order of priority)
const CONFIG_FILE_NAMES: &[&str] = &[
    ".nix-let-fn-linter.toml",
    "nix-let-fn-linter.toml",
];

#[derive(Parser, Debug)]
#[command(name = "nix-let-fn-linter")]
#[command(about = "Detects function definitions in Nix let-in blocks")]
#[command(version)]
#[command(after_help = "CONFIGURATION FILE:
    The linter looks for configuration files in the following order:
    1. File specified with --config
    2. .nix-let-fn-linter.toml in current directory
    3. nix-let-fn-linter.toml in current directory
    4. Same files in parent directories (up to root)

    Example config file:

    [paths]
    include = [\"src/**/*.nix\", \"modules/**/*.nix\"]
    exclude = [\"**/test/**\", \"**/fixtures/**\"]

    [output]
    format = \"text\"
    no_color = false

    [lint]
    strict = false
    skip_directives = [\"nix-let-fn-linter:disable\", \"noqa: let-fn\"]
")]
struct Cli {
    /// Paths to Nix files or directories to lint
    #[arg(required_unless_present_any = ["config", "init", "show_config"])]
    paths: Vec<PathBuf>,

    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Exit with error code if any warnings are found
    #[arg(long)]
    strict: bool,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    /// Glob patterns to include (can be specified multiple times)
    #[arg(short = 'I', long = "include")]
    include: Vec<String>,

    /// Glob patterns to exclude (can be specified multiple times)
    #[arg(short = 'E', long = "exclude")]
    exclude: Vec<String>,

    /// Generate a default config file
    #[arg(long)]
    init: bool,

    /// Show resolved configuration and exit
    #[arg(long)]
    show_config: bool,
}

#[derive(ValueEnum, Clone, Debug, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Config {
    #[serde(default)]
    paths: PathsConfig,
    #[serde(default)]
    output: OutputConfig,
    #[serde(default)]
    lint: LintConfig,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct PathsConfig {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct OutputConfig {
    #[serde(default)]
    format: OutputFormat,
    #[serde(default)]
    no_color: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct LintConfig {
    #[serde(default)]
    strict: bool,
    #[serde(default = "default_skip_directives")]
    skip_directives: Vec<String>,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            strict: false,
            skip_directives: default_skip_directives(),
        }
    }
}

fn default_skip_directives() -> Vec<String> {
    DEFAULT_SKIP_DIRECTIVES.iter().map(|s| s.to_string()).collect()
}

#[derive(Serialize, Debug, Clone)]
struct Warning {
    file: String,
    line: usize,
    column: usize,
    binding_name: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    nesting_depth: Option<usize>,
    #[serde(skip)]
    line_content: String,
}

#[derive(Serialize, Debug)]
struct LintResult {
    total_files: usize,
    total_warnings: usize,
    warnings: Vec<Warning>,
}

/// Resolved configuration after merging CLI and config file
#[derive(Debug)]
struct ResolvedConfig {
    paths: Vec<PathBuf>,
    include_patterns: Vec<Pattern>,
    exclude_patterns: Vec<Pattern>,
    format: OutputFormat,
    strict: bool,
    no_color: bool,
    skip_directives: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    // Handle --init flag
    if cli.init {
        generate_default_config();
        return;
    }

    // Load and merge configuration
    let config = match load_config(&cli) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    // Handle color settings early
    if config.no_color {
        colored::control::set_override(false);
    }

    // Handle --show-config flag
    if cli.show_config {
        show_resolved_config(&config);
        return;
    }

    // Run the linter
    let result = run_linter(&config);

    match config.format {
        OutputFormat::Text => print_text_output(&result),
        OutputFormat::Json => print_json_output(&result),
    }

    if config.strict && result.total_warnings > 0 {
        std::process::exit(1);
    }
}

fn generate_default_config() {
    let config = Config {
        paths: PathsConfig {
            include: vec!["**/*.nix".to_string()],
            exclude: vec!["**/result/**".to_string(), "**/node_modules/**".to_string()],
        },
        output: OutputConfig {
            format: OutputFormat::Text,
            no_color: false,
        },
        lint: LintConfig {
            strict: false,
            skip_directives: default_skip_directives(),
        },
    };

    let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize config");
    println!("{}", "# nix-let-fn-linter configuration".dimmed());
    println!("{}", toml_str);
    println!();
    println!(
        "{} Save this to {} or {}",
        "hint:".cyan().bold(),
        ".nix-let-fn-linter.toml".bold(),
        "nix-let-fn-linter.toml".bold()
    );
}

fn show_resolved_config(config: &ResolvedConfig) {
    println!("{}", "Resolved Configuration:".bold().underline());
    println!();
    println!("{}:", "paths".cyan());
    for path in &config.paths {
        println!("  - {}", path.display());
    }
    println!();
    println!("{}:", "include_patterns".cyan());
    if config.include_patterns.is_empty() {
        println!("  {}", "(all .nix files)".dimmed());
    } else {
        for pattern in &config.include_patterns {
            println!("  - {}", pattern);
        }
    }
    println!();
    println!("{}:", "exclude_patterns".cyan());
    if config.exclude_patterns.is_empty() {
        println!("  {}", "(none)".dimmed());
    } else {
        for pattern in &config.exclude_patterns {
            println!("  - {}", pattern);
        }
    }
    println!();
    println!("{}: {:?}", "format".cyan(), config.format);
    println!("{}: {}", "strict".cyan(), config.strict);
    println!("{}: {}", "no_color".cyan(), config.no_color);
    println!();
    println!("{}:", "skip_directives".cyan());
    for directive in &config.skip_directives {
        println!("  - {}", directive);
    }
}

fn find_config_file(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        for name in CONFIG_FILE_NAMES {
            let config_path = current.join(name);
            if config_path.exists() {
                return Some(config_path);
            }
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn load_config(cli: &Cli) -> Result<ResolvedConfig, String> {
    // Try to load config file
    let file_config = if let Some(ref config_path) = cli.config {
        // Explicit config file specified
        if !config_path.exists() {
            return Err(format!("Config file not found: {}", config_path.display()));
        }
        let content = std::fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        Some(toml::from_str::<Config>(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?)
    } else {
        // Search for config file
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        find_config_file(&cwd).and_then(|path| {
            std::fs::read_to_string(&path).ok().and_then(|content| {
                toml::from_str::<Config>(&content).ok()
            })
        })
    };

    let config = file_config.unwrap_or_default();

    // Merge CLI args with config file (CLI takes precedence)
    let paths: Vec<PathBuf> = if cli.paths.is_empty() {
        // Use include patterns from config if no paths specified
        if config.paths.include.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            // Convert glob patterns to paths
            config.paths.include.iter()
                .filter_map(|p| glob::glob(p).ok())
                .flatten()
                .filter_map(|p| p.ok())
                .collect()
        }
    } else {
        cli.paths.clone()
    };

    // Merge include patterns
    let include_strs: Vec<String> = if cli.include.is_empty() {
        config.paths.include.clone()
    } else {
        cli.include.clone()
    };

    let include_patterns: Vec<Pattern> = include_strs.iter()
        .filter_map(|s| Pattern::new(s).ok())
        .collect();

    // Merge exclude patterns
    let exclude_strs: Vec<String> = if cli.exclude.is_empty() {
        config.paths.exclude.clone()
    } else {
        cli.exclude.clone()
    };

    let exclude_patterns: Vec<Pattern> = exclude_strs.iter()
        .filter_map(|s| Pattern::new(s).ok())
        .collect();

    // Determine format (CLI overrides config)
    let format = cli.format.clone().unwrap_or(config.output.format);

    // strict: CLI flag OR config value
    let strict = cli.strict || config.lint.strict;

    // no_color: CLI flag OR config value
    let no_color = cli.no_color || config.output.no_color;

    // Skip directives from config (or defaults)
    let skip_directives = config.lint.skip_directives;

    Ok(ResolvedConfig {
        paths,
        include_patterns,
        exclude_patterns,
        format,
        strict,
        no_color,
        skip_directives,
    })
}

fn run_linter(config: &ResolvedConfig) -> LintResult {
    let mut all_warnings = Vec::new();
    let mut total_files = 0;

    for path in &config.paths {
        if path.is_file() {
            if should_lint_file(path, &config.include_patterns, &config.exclude_patterns) {
                total_files += 1;
                if let Ok(warnings) = lint_file(path, &config.skip_directives) {
                    all_warnings.extend(warnings);
                }
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let entry_path = entry.path();
                if entry_path.is_file()
                    && should_lint_file(entry_path, &config.include_patterns, &config.exclude_patterns)
                {
                    total_files += 1;
                    if let Ok(warnings) = lint_file(entry_path, &config.skip_directives) {
                        all_warnings.extend(warnings);
                    }
                }
            }
        }
    }

    LintResult {
        total_files,
        total_warnings: all_warnings.len(),
        warnings: all_warnings,
    }
}

fn should_lint_file(path: &Path, include: &[Pattern], exclude: &[Pattern]) -> bool {
    // Must be a .nix file
    if path.extension().map_or(true, |ext| ext != "nix") {
        return false;
    }

    let path_str = path.to_string_lossy();

    // Check exclude patterns first
    for pattern in exclude {
        if pattern.matches(&path_str) {
            return false;
        }
    }

    // If no include patterns, include all .nix files
    if include.is_empty() {
        return true;
    }

    // Check include patterns
    for pattern in include {
        if pattern.matches(&path_str) {
            return true;
        }
    }

    false
}

fn lint_file(path: &Path, skip_directives: &[String]) -> Result<Vec<Warning>, std::io::Error> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    let parse = rnix::Root::parse(&content);
    let root = parse.tree();

    let mut warnings = Vec::new();
    let file_str = path.to_string_lossy().to_string();

    visit_node(root.syntax(), &content, &lines, &file_str, &mut warnings, 0, skip_directives);

    Ok(warnings)
}

fn visit_node(
    node: &SyntaxNode,
    content: &str,
    lines: &[&str],
    file: &str,
    warnings: &mut Vec<Warning>,
    let_depth: usize,
    skip_directives: &[String],
) {
    if node.kind() == SyntaxKind::NODE_LET_IN {
        if let Some(let_in) = ast::LetIn::cast(node.clone()) {
            check_let_in_bindings(&let_in, content, lines, file, warnings, let_depth + 1, skip_directives);
        }
        return;
    }

    for child in node.children() {
        visit_node(&child, content, lines, file, warnings, let_depth, skip_directives);
    }
}

fn check_let_in_bindings(
    let_in: &ast::LetIn,
    content: &str,
    lines: &[&str],
    file: &str,
    warnings: &mut Vec<Warning>,
    depth: usize,
    skip_directives: &[String],
) {
    for entry in let_in.attrpath_values() {
        if let Some(value) = entry.value() {
            if is_lambda(&value) {
                let binding_name = entry
                    .attrpath()
                    .map(|ap| {
                        ap.attrs()
                            .map(|attr| attr_to_string(&attr))
                            .collect::<Vec<_>>()
                            .join(".")
                    })
                    .unwrap_or_else(|| "<unknown>".to_string());

                let start_offset: usize = entry.syntax().text_range().start().into();
                let (line, column) = offset_to_line_col(content, start_offset);

                let line_content = if line > 0 && line <= lines.len() {
                    lines[line - 1].to_string()
                } else {
                    String::new()
                };

                // Check for skip directive
                if has_skip_directive(&line_content, skip_directives) {
                    continue;
                }
                if line > 1 && line <= lines.len() {
                    if has_skip_directive(lines[line - 2], skip_directives) {
                        continue;
                    }
                }

                warnings.push(Warning {
                    file: file.to_string(),
                    line,
                    column,
                    binding_name: binding_name.clone(),
                    message: "Function defined in let-in block".to_string(),
                    nesting_depth: if depth > 1 { Some(depth) } else { None },
                    line_content,
                });
            }

            visit_node(value.syntax(), content, lines, file, warnings, depth, skip_directives);
        }
    }

    if let Some(body) = let_in.body() {
        visit_node(body.syntax(), content, lines, file, warnings, depth, skip_directives);
    }
}

fn has_skip_directive(line: &str, skip_directives: &[String]) -> bool {
    let line_lower = line.to_lowercase();
    skip_directives.iter().any(|directive| line_lower.contains(&directive.to_lowercase()))
}

fn is_lambda(expr: &ast::Expr) -> bool {
    matches!(expr, ast::Expr::Lambda(_))
}

fn attr_to_string(attr: &ast::Attr) -> String {
    match attr {
        ast::Attr::Ident(ident) => ident.syntax().text().to_string(),
        ast::Attr::Str(s) => s.syntax().text().to_string(),
        ast::Attr::Dynamic(d) => d.syntax().text().to_string(),
    }
}

fn offset_to_line_col(content: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in content.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn print_text_output(result: &LintResult) {
    if result.warnings.is_empty() {
        println!(
            "{} No function definitions found in let-in blocks ({} files checked)",
            "ok:".green().bold(),
            result.total_files
        );
        return;
    }

    // Group warnings by file
    let mut warnings_by_file: std::collections::HashMap<&str, Vec<&Warning>> =
        std::collections::HashMap::new();
    for warning in &result.warnings {
        warnings_by_file
            .entry(&warning.file)
            .or_default()
            .push(warning);
    }

    for (file, file_warnings) in &warnings_by_file {
        println!();
        println!("{}", file.underline().bold());

        for warning in file_warnings {
            let location = format!("  {}:{}", warning.line, warning.column);
            let depth_info = warning
                .nesting_depth
                .map(|d| format!(" {}", format!("(depth: {})", d).dimmed()))
                .unwrap_or_default();

            println!(
                "{} {} {} `{}`{}",
                location.cyan(),
                "warning:".yellow().bold(),
                warning.message,
                warning.binding_name.magenta().bold(),
                depth_info
            );

            if !warning.line_content.is_empty() {
                let line_num = format!("{:>4} |", warning.line);
                println!("     {}", "|".blue());
                println!("{} {}", line_num.blue(), warning.line_content);

                let padding = " ".repeat(warning.column - 1);
                let underline_len = warning.binding_name.len().max(1);
                let underline = "^".repeat(underline_len);
                println!(
                    "     {} {}{}",
                    "|".blue(),
                    padding,
                    underline.yellow().bold()
                );
            }
        }
    }

    println!();
    let warning_text = if result.total_warnings == 1 { "warning" } else { "warnings" };
    let file_text = if result.total_files == 1 { "file" } else { "files" };

    println!(
        "{} {} {} found in {} {}",
        "Found:".yellow().bold(),
        result.total_warnings.to_string().yellow().bold(),
        warning_text,
        result.total_files,
        file_text
    );

    println!();
    println!(
        "{} To disable a warning, add a comment above the line:",
        "hint:".cyan().bold()
    );
    println!(
        "      {} or {}",
        "# nix-let-fn-linter:disable".dimmed(),
        "# noqa: let-fn".dimmed()
    );
}

fn print_json_output(result: &LintResult) {
    println!("{}", serde_json::to_string_pretty(result).unwrap());
}
