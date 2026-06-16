use clap::{Parser, Subcommand};
use anyhow::Result;
use ll_core::ai::AiAnalyzer;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "loglens", about = "AI-powered log analysis tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Tail and analyze a log file or Docker container
    Watch {
        /// File path, directory, or docker://container-name
        target: String,
        /// Show only entries at this level or above (trace/debug/info/warn/error/fatal)
        #[arg(short, long, default_value = "info")]
        level: String,
        /// Explain errors with AI automatically
        #[arg(short, long)]
        ai: bool,
    },
    /// Search logs in the database
    Search {
        /// Full-text search query
        query: String,
        /// Limit results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Show error clusters
    Clusters {
        /// Show top N clusters
        #[arg(short, long, default_value = "20")]
        top: usize,
    },
    /// Run AI root-cause analysis on a cluster
    Analyze {
        /// Cluster ID
        cluster_id: String,
    },
    /// Export logs to file
    Export {
        /// Output path (.json or .md)
        output: String,
        /// Optional: filter by level
        #[arg(short, long)]
        level: Option<String>,
        /// Include AI summary
        #[arg(long)]
        ai_summary: bool,
    },
    /// Configure LogLens (set API key etc.)
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set the Claude API key
    SetKey { key: String },
    /// Check if API key is set
    Check,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("loglens=info".parse()?))
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let db_path = get_db_path();

    match cli.command {
        Cmd::Watch { target, level, ai } => {
            cmd_watch(&target, &level, ai, &db_path).await?;
        }
        Cmd::Search { query, limit } => {
            cmd_search(&query, limit, &db_path).await?;
        }
        Cmd::Clusters { top } => {
            cmd_clusters(top, &db_path).await?;
        }
        Cmd::Analyze { cluster_id } => {
            cmd_analyze(&cluster_id, &db_path).await?;
        }
        Cmd::Export { output, level, ai_summary } => {
            cmd_export(&output, level.as_deref(), ai_summary, &db_path).await?;
        }
        Cmd::Config { action } => {
            cmd_config(action).await?;
        }
    }

    Ok(())
}

fn get_db_path() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("loglens")
        .join("loglens.db")
}

async fn cmd_watch(target: &str, min_level: &str, auto_ai: bool, db_path: &std::path::Path) -> Result<()> {
    use ll_core::models::log_entry::{LogSource, LogSourceKind, LogLevel};
    use ll_core::collector::LogCollector;
    use tokio::sync::mpsc;

    std::fs::create_dir_all(db_path.parent().unwrap())?;
    let pool = ll_core::db::open(db_path).await?;

    let (tx, mut rx) = mpsc::channel(1024);
    let grouper = std::sync::Arc::new(ll_core::clustering::ClusterGrouper::new());
    let collector = LogCollector::new(tx, grouper);

    let (source, kind) = if let Some(container) = target.strip_prefix("docker://") {
        let name = container.to_string();
        let kind = LogSourceKind::DockerContainer {
            container_id: name.clone(),
            name: name.clone(),
        };
        (LogSource::new(format!("docker:{}", name), kind.clone()), kind)
    } else {
        let path = target.to_string();
        let kind = LogSourceKind::File { path: path.clone() };
        (LogSource::new(path, kind.clone()), kind)
    };

    let min = LogLevel::from_str(min_level);
    collector.watch(source).await?;

    println!("Watching {}  [Ctrl+C to stop]", target);

    let api_key = get_api_key().unwrap_or_default();
    let analyzer: Option<Box<dyn ll_core::ai::AiAnalyzer>> = if auto_ai && !api_key.is_empty() {
        Some(Box::new(ll_core::ai::claude::ClaudeAnalyzer::new(api_key)))
    } else {
        None
    };

    while let Some(entry) = rx.recv().await {
        if entry.level < min {
            continue;
        }

        let level_str = format!("{:?}", entry.level);
        let svc = entry.service.as_deref().unwrap_or("-");
        println!(
            "{} [{:<5}] [{}] {}",
            entry.timestamp.format("%H:%M:%S%.3f"),
            level_str, svc, entry.message
        );

        if let Some(ref st) = entry.stacktrace {
            for line in st.iter().take(5) {
                println!("  {}", line);
            }
        }

        if auto_ai {
            if let Some(ref ai) = analyzer {
                if entry.level >= ll_core::models::log_entry::LogLevel::Error {
                    match ai.explain_entry(&entry).await {
                        Ok(expl) => {
                            println!("  AI: {}", expl.what);
                            if let Some(fix) = expl.fix_suggestions.first() {
                                println!("  Fix: {}", fix);
                            }
                        }
                        Err(e) => tracing::warn!("AI explain failed: {}", e),
                    }
                }
            }
        }

        ll_core::db::queries::insert_entry(&pool, &entry).await.ok();
    }

    Ok(())
}

async fn cmd_search(query: &str, limit: usize, db_path: &std::path::Path) -> Result<()> {
    use ll_core::query::QueryEngine;
    use ll_core::models::query::{QueryRequest, QueryFilter};

    let pool = ll_core::db::open(db_path).await?;
    let engine = QueryEngine::new(pool);

    let req = QueryRequest {
        filter: QueryFilter { text: Some(query.to_string()), limit: Some(limit), ..Default::default() },
        highlight: false,
        sort_desc: true,
        ai_query: None,
    };

    let result = engine.query(&req).await?;
    println!("Found {} entries (showing {}), took {}ms
", result.total, result.entries.len(), result.took_ms);

    for entry in &result.entries {
        let svc = entry.service.as_deref().unwrap_or("-");
        println!(
            "{} [{:?}] [{}] {}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            entry.level, svc, entry.message
        );
    }

    Ok(())
}

async fn cmd_clusters(top: usize, db_path: &std::path::Path) -> Result<()> {
    let pool = ll_core::db::open(db_path).await?;
    let clusters = ll_core::db::queries::list_clusters(&pool, top as i64).await?;

    println!("{:>8}  {:>5}  {:<16}  Template", "Count", "Level", "First seen");
    println!("{}", "-".repeat(80));

    for c in clusters {
        println!(
            "{:>8}  {:>5}  {:<16}  {}",
            c.count,
            format!("{:?}", c.level),
            c.first_seen.format("%m-%d %H:%M:%S").to_string(),
            c.template.chars().take(60).collect::<String>()
        );
    }

    Ok(())
}

async fn cmd_analyze(cluster_id: &str, db_path: &std::path::Path) -> Result<()> {
    let pool = ll_core::db::open(db_path).await?;
    let clusters = ll_core::db::queries::list_clusters(&pool, 1000).await?;

    let cluster = clusters.into_iter()
        .find(|c| c.id == cluster_id || c.id.starts_with(cluster_id))
        .ok_or_else(|| anyhow::anyhow!("Cluster not found: {}", cluster_id))?;

    let api_key = get_api_key()?;
    let analyzer = ll_core::ai::claude::ClaudeAnalyzer::new(api_key);

    println!("Analyzing cluster: {}", cluster.template);
    println!("Occurrences: {}  Level: {:?}", cluster.count, cluster.level);
    println!("Running AI root-cause analysis...
");

    let report = analyzer.root_cause(&cluster, &[]).await?;

    println!("## {}
", report.title);
    println!("**Root Cause:** {}
", report.root_cause);

    if !report.contributing_factors.is_empty() {
        println!("**Contributing Factors:**");
        for f in &report.contributing_factors {
            println!("  - {}", f);
        }
        println!();
    }

    println!("**Fix Steps:**");
    for step in &report.fix_suggestions {
        println!("  {}. {}", step.step, step.title);
        println!("     {}", step.description);
        if let Some(ref cmd) = step.command {
            println!("     $ {}", cmd);
        }
    }

    println!("
Confidence: {:.0}%", report.confidence * 100.0);

    Ok(())
}

async fn cmd_export(
    output: &str,
    _level: Option<&str>,
    _ai_summary: bool,
    db_path: &std::path::Path,
) -> Result<()> {
    use ll_core::query::QueryEngine;
    use ll_core::models::query::{QueryRequest, QueryFilter};

    let pool = ll_core::db::open(db_path).await?;
    let engine = QueryEngine::new(pool.clone());

    let req = QueryRequest {
        filter: QueryFilter { limit: Some(5000), ..Default::default() },
        ..Default::default()
    };
    let result = engine.query(&req).await?;

    let content = if output.ends_with(".md") {
        ll_core::export::export_markdown(&result.entries, None)
    } else {
        ll_core::export::export_json(&result.entries, None)?
    };

    std::fs::write(output, &content)?;
    println!("Exported {} entries to {}", result.entries.len(), output);

    Ok(())
}

async fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::SetKey { key } => {
            let kr = keyring::Entry::new("loglens", "claude_api_key")?;
            kr.set_password(&key)?;
            println!("API key stored successfully.");
        }
        ConfigAction::Check => {
            match get_api_key() {
                Ok(_) => println!("API key: set"),
                Err(_) => println!("API key: not set  (use `loglens config set-key <key>`)"),
            }
        }
    }
    Ok(())
}

fn get_api_key() -> Result<String> {
    let kr = keyring::Entry::new("loglens", "claude_api_key")?;
    Ok(kr.get_password()?)
}
