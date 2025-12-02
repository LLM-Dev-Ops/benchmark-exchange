//! LLM Benchmark Exchange CLI
//!
//! Command-line interface for managing benchmarks, submissions, and results.

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use llm_benchmark_cli::commands::{
    auth, benchmark, init, leaderboard, proposal, run, submit, CommandContext,
};
use llm_benchmark_cli::config::Config;
use llm_benchmark_cli::output::OutputFormat;

/// Output format for CLI commands
#[derive(Copy, Clone, Debug, Default, ValueEnum)]
pub enum CliOutputFormat {
    /// JSON output
    Json,
    /// Table output (default)
    #[default]
    Table,
    /// Plain text output
    Plain,
}

impl From<CliOutputFormat> for OutputFormat {
    fn from(f: CliOutputFormat) -> Self {
        match f {
            CliOutputFormat::Json => OutputFormat::Json,
            CliOutputFormat::Table => OutputFormat::Table,
            CliOutputFormat::Plain => OutputFormat::Plain,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "llm-benchmark")]
#[command(author, version, about = "LLM Benchmark Exchange CLI")]
#[command(long_about = "Command-line interface for the LLM Benchmark Exchange platform.\n\n\
    Manage benchmarks, submit evaluation results, view leaderboards, and participate in governance.")]
#[command(propagate_version = true)]
struct Cli {
    /// Output format
    #[arg(short = 'o', long, global = true, value_enum, default_value = "table")]
    format: CliOutputFormat,

    /// API endpoint URL (overrides config)
    #[arg(long, global = true, env = "LLM_BENCHMARK_API_URL")]
    api_url: Option<String>,

    /// Authentication token (overrides config)
    #[arg(long, global = true, env = "LLM_BENCHMARK_TOKEN")]
    token: Option<String>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Authentication commands
    #[command(alias = "a")]
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Benchmark management commands
    #[command(alias = "b", alias = "bench")]
    Benchmark {
        #[command(subcommand)]
        command: BenchmarkCommands,
    },

    /// Submission commands
    #[command(alias = "s", alias = "sub")]
    Submit {
        #[command(subcommand)]
        command: SubmitCommands,
    },

    /// Leaderboard commands
    #[command(alias = "l", alias = "lb")]
    Leaderboard {
        #[command(subcommand)]
        command: LeaderboardCommands,
    },

    /// Governance proposal commands
    #[command(alias = "p", alias = "prop")]
    Proposal {
        #[command(subcommand)]
        command: ProposalCommands,
    },

    /// Initialize a new benchmark project
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,

        /// Project directory (defaults to current directory)
        #[arg(short, long)]
        directory: Option<String>,

        /// Use non-interactive mode with defaults
        #[arg(long)]
        non_interactive: bool,
    },

    /// Generate template files
    Scaffold {
        /// Template type (test-case, results, benchmark, evaluator)
        #[arg(value_name = "TYPE")]
        template: String,

        /// Output file name
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Show current configuration
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Run benchmark suite
    #[command(alias = "r")]
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// Reset configuration to defaults
    Reset,
}

#[derive(Subcommand, Debug)]
enum AuthCommands {
    /// Login to the LLM Benchmark Exchange
    Login {
        /// Authentication token (for non-interactive login)
        #[arg(short, long)]
        token: Option<String>,

        /// API key (alternative to token)
        #[arg(long)]
        api_key: Option<String>,
    },

    /// Logout from the LLM Benchmark Exchange
    Logout,

    /// Show current user information
    Whoami,

    /// Refresh authentication token
    Refresh,

    /// Show authentication status
    Status,
}

#[derive(Subcommand, Debug)]
enum BenchmarkCommands {
    /// List benchmarks
    List {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,

        /// Search query
        #[arg(short, long)]
        query: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Result offset for pagination
        #[arg(long, default_value = "0")]
        offset: u32,
    },

    /// Show benchmark details
    Show {
        /// Benchmark ID or slug
        #[arg(value_name = "ID")]
        id: String,

        /// Show version history
        #[arg(long)]
        versions: bool,
    },

    /// Create a new benchmark
    Create {
        /// Path to benchmark definition file (YAML or JSON)
        #[arg(value_name = "FILE")]
        file: String,

        /// Submit for review immediately
        #[arg(long)]
        submit: bool,
    },

    /// Update an existing benchmark
    Update {
        /// Benchmark ID
        #[arg(value_name = "ID")]
        id: String,

        /// Path to updated definition file
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Submit a benchmark for review
    SubmitForReview {
        /// Benchmark ID
        #[arg(value_name = "ID")]
        id: String,

        /// Reason for submission
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Validate a benchmark definition file
    Validate {
        /// Path to benchmark definition file
        #[arg(value_name = "FILE")]
        file: String,

        /// Strict validation mode
        #[arg(long)]
        strict: bool,
    },

    /// Download benchmark test cases
    Download {
        /// Benchmark ID or slug
        #[arg(value_name = "ID")]
        id: String,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,
    },

    /// Show benchmark statistics
    Stats {
        /// Benchmark ID or slug
        #[arg(value_name = "ID")]
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum SubmitCommands {
    /// Submit results to a benchmark
    #[command(alias = "new")]
    Submit {
        /// Benchmark ID
        #[arg(short, long)]
        benchmark: String,

        /// Path to results file
        #[arg(short, long)]
        results: String,

        /// Model name
        #[arg(short, long)]
        model: String,

        /// Model version
        #[arg(short = 'V', long)]
        version: String,

        /// Model provider
        #[arg(short, long)]
        provider: Option<String>,

        /// Visibility (public, private, unlisted)
        #[arg(long, default_value = "public")]
        visibility: String,

        /// Additional notes
        #[arg(long)]
        notes: Option<String>,
    },

    /// Show submission details
    Show {
        /// Submission ID
        #[arg(value_name = "ID")]
        id: String,

        /// Show full results
        #[arg(long)]
        full: bool,
    },

    /// List submissions
    List {
        /// Filter by benchmark ID
        #[arg(short, long)]
        benchmark: Option<String>,

        /// Filter by model name
        #[arg(short, long)]
        model: Option<String>,

        /// Filter by verification status
        #[arg(long)]
        verification: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Result offset for pagination
        #[arg(long, default_value = "0")]
        offset: u32,
    },

    /// Request verification for a submission
    RequestVerification {
        /// Submission ID
        #[arg(value_name = "ID")]
        id: String,

        /// Verification level requested
        #[arg(long, default_value = "platform")]
        level: String,
    },

    /// Cancel a pending submission
    Cancel {
        /// Submission ID
        #[arg(value_name = "ID")]
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum LeaderboardCommands {
    /// Show leaderboard for a benchmark
    Show {
        /// Benchmark ID
        #[arg(value_name = "BENCHMARK_ID")]
        benchmark_id: String,

        /// Number of entries to show
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Filter by verification level
        #[arg(long)]
        verified_only: bool,
    },

    /// Compare two models
    Compare {
        /// Benchmark ID
        #[arg(short, long)]
        benchmark: String,

        /// First model name
        #[arg(value_name = "MODEL1")]
        model1: String,

        /// Second model name
        #[arg(value_name = "MODEL2")]
        model2: String,

        /// Show detailed metric comparison
        #[arg(long)]
        detailed: bool,
    },

    /// Export leaderboard data
    Export {
        /// Benchmark ID
        #[arg(value_name = "BENCHMARK_ID")]
        benchmark_id: String,

        /// Output format (json, csv)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Watch leaderboard for updates
    Watch {
        /// Benchmark ID
        #[arg(value_name = "BENCHMARK_ID")]
        benchmark_id: String,

        /// Refresh interval in seconds
        #[arg(short, long, default_value = "30")]
        interval: u64,
    },
}

#[derive(Subcommand, Debug)]
enum ProposalCommands {
    /// List proposals
    List {
        /// Filter by status (draft, voting, approved, rejected)
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by type
        #[arg(short = 't', long)]
        proposal_type: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: u32,
    },

    /// Show proposal details
    Show {
        /// Proposal ID
        #[arg(value_name = "ID")]
        id: String,

        /// Show comments
        #[arg(long)]
        comments: bool,

        /// Show vote breakdown
        #[arg(long)]
        votes: bool,
    },

    /// Create a new proposal
    Create {
        /// Proposal type (new-benchmark, update-benchmark, deprecate-benchmark, governance)
        #[arg(short, long)]
        r#type: String,

        /// Path to proposal content file
        #[arg(short, long)]
        file: Option<String>,

        /// Proposal title
        #[arg(long)]
        title: Option<String>,

        /// Submit for voting immediately
        #[arg(long)]
        submit: bool,
    },

    /// Vote on a proposal
    Vote {
        /// Proposal ID
        #[arg(value_name = "ID")]
        id: String,

        /// Vote (approve, reject, abstain)
        #[arg(short, long)]
        vote: String,

        /// Reason for vote
        #[arg(short, long)]
        reason: Option<String>,
    },

    /// Comment on a proposal
    Comment {
        /// Proposal ID
        #[arg(value_name = "ID")]
        id: String,

        /// Comment message
        #[arg(short, long)]
        message: Option<String>,

        /// Reply to a specific comment
        #[arg(long)]
        reply_to: Option<String>,
    },

    /// Withdraw a proposal
    Withdraw {
        /// Proposal ID
        #[arg(value_name = "ID")]
        id: String,

        /// Reason for withdrawal
        #[arg(short, long)]
        reason: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum RunCommands {
    /// Run all benchmarks
    All {
        /// Output directory for results
        #[arg(short, long)]
        output: Option<String>,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Run a specific benchmark target
    Single {
        /// Benchmark target ID
        #[arg(value_name = "TARGET_ID")]
        target_id: String,

        /// Output directory for results
        #[arg(short, long)]
        output: Option<String>,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// List available benchmark targets
    List,

    /// Show benchmark results summary
    Summary {
        /// Directory containing benchmark results
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn generate_completions(shell: clap_complete::Shell) {
    use clap::CommandFactory;
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "llm-benchmark", &mut std::io::stdout());
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Setup colored output
    if cli.no_color {
        colored::control::set_override(false);
    }

    // Initialize tracing
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::WARN
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into()),
        )
        .init();

    // Handle completions command early (doesn't need config)
    if let Commands::Completions { shell } = cli.command {
        generate_completions(shell);
        return Ok(());
    }

    // Load configuration
    let mut config = Config::load()?;

    // Override config with CLI arguments
    if let Some(api_url) = &cli.api_url {
        config.api_endpoint = api_url.clone();
    }
    if let Some(token) = &cli.token {
        config.auth_token = Some(token.clone());
    }

    // Set output format
    config.output_format = cli.format.into();

    let mut ctx = CommandContext::new(config)?;

    // Execute command
    let result = match cli.command {
        Commands::Auth { command } => match command {
            AuthCommands::Login { token, api_key } => {
                auth::login(&mut ctx, token.or(api_key)).await
            }
            AuthCommands::Logout => auth::logout(&mut ctx).await,
            AuthCommands::Whoami => auth::whoami(&ctx).await,
            AuthCommands::Refresh => {
                println!("Token refresh not yet implemented");
                Ok(())
            }
            AuthCommands::Status => auth::whoami(&ctx).await,
        },

        Commands::Benchmark { command } => match command {
            BenchmarkCommands::List {
                category,
                status,
                query: _,
                limit: _,
                offset: _,
            } => benchmark::list(&ctx, category, status).await,
            BenchmarkCommands::Show { id, versions: _ } => benchmark::show(&ctx, id).await,
            BenchmarkCommands::Create { file, submit: _ } => benchmark::create(&ctx, file).await,
            BenchmarkCommands::Update { id, file } => benchmark::update(&ctx, id, file).await,
            BenchmarkCommands::SubmitForReview { id, message: _ } => {
                benchmark::submit_for_review(&ctx, id).await
            }
            BenchmarkCommands::Validate { file, strict: _ } => benchmark::validate(file).await,
            BenchmarkCommands::Download { id: _, output: _ } => {
                println!("Download command not yet implemented");
                Ok(())
            }
            BenchmarkCommands::Stats { id: _ } => {
                println!("Stats command not yet implemented");
                Ok(())
            }
        },

        Commands::Submit { command } => match command {
            SubmitCommands::Submit {
                benchmark,
                results,
                model,
                version,
                provider: _,
                visibility: _,
                notes: _,
            } => submit::submit(&ctx, benchmark, results, model, version).await,
            SubmitCommands::Show { id, full: _ } => submit::show(&ctx, id).await,
            SubmitCommands::List {
                benchmark,
                model: _,
                verification: _,
                limit: _,
                offset: _,
            } => submit::list(&ctx, benchmark).await,
            SubmitCommands::RequestVerification { id, level: _ } => {
                submit::request_verification(&ctx, id).await
            }
            SubmitCommands::Cancel { id: _ } => {
                println!("Cancel command not yet implemented");
                Ok(())
            }
        },

        Commands::Leaderboard { command } => match command {
            LeaderboardCommands::Show {
                benchmark_id,
                limit: _,
                verified_only: _,
            } => leaderboard::show(&ctx, benchmark_id).await,
            LeaderboardCommands::Compare {
                benchmark,
                model1,
                model2,
                detailed: _,
            } => leaderboard::compare(&ctx, benchmark, model1, model2).await,
            LeaderboardCommands::Export {
                benchmark_id,
                format,
                output,
            } => leaderboard::export(&ctx, benchmark_id, format, output).await,
            LeaderboardCommands::Watch {
                benchmark_id: _,
                interval: _,
            } => {
                println!("Watch command not yet implemented");
                Ok(())
            }
        },

        Commands::Proposal { command } => match command {
            ProposalCommands::List {
                status,
                proposal_type: _,
                limit: _,
            } => proposal::list(&ctx, status).await,
            ProposalCommands::Show {
                id,
                comments: _,
                votes: _,
            } => proposal::show(&ctx, id).await,
            ProposalCommands::Create {
                r#type,
                file,
                title: _,
                submit: _,
            } => proposal::create(&ctx, r#type, file).await,
            ProposalCommands::Vote { id, vote, reason: _ } => proposal::vote(&ctx, id, vote).await,
            ProposalCommands::Comment {
                id,
                message,
                reply_to: _,
            } => proposal::comment(&ctx, id, message).await,
            ProposalCommands::Withdraw { id: _, reason: _ } => {
                println!("Withdraw command not yet implemented");
                Ok(())
            }
        },

        Commands::Init {
            name,
            directory: _,
            non_interactive: _,
        } => init::init(name).await,

        Commands::Scaffold { template, output: _ } => init::scaffold(template).await,

        Commands::Config { command } => {
            match command {
                Some(ConfigCommands::Show) | None => {
                    println!("Current configuration:");
                    println!("  API Endpoint: {}", ctx.config.api_endpoint);
                    println!(
                        "  Auth Token: {}",
                        if ctx.config.auth_token.is_some() {
                            "***"
                        } else {
                            "(not set)"
                        }
                    );
                    println!("  Output Format: {:?}", ctx.config.output_format);
                }
                Some(ConfigCommands::Set { key, value }) => {
                    println!("Setting {} = {}", key, value);
                    // TODO: Implement config set
                }
                Some(ConfigCommands::Get { key }) => {
                    println!("Getting {}", key);
                    // TODO: Implement config get
                }
                Some(ConfigCommands::Reset) => {
                    println!("Resetting configuration to defaults");
                    // TODO: Implement config reset
                }
            }
            Ok(())
        }

        Commands::Completions { .. } => {
            // Already handled above
            Ok(())
        }

        Commands::Run { command } => match command {
            RunCommands::All { output, json } => {
                run::run_all(output.map(std::path::PathBuf::from), json).await
            }
            RunCommands::Single {
                target_id,
                output,
                json,
            } => run::run_single(target_id, output.map(std::path::PathBuf::from), json).await,
            RunCommands::List => run::list().await,
            RunCommands::Summary { output } => {
                run::show_summary(output.map(std::path::PathBuf::from)).await
            }
        },
    };

    // Handle errors
    if let Err(e) = result {
        use colored::Colorize;
        eprintln!("{} {}", "Error:".red().bold(), e);
        if cli.verbose {
            eprintln!("\n{}", "Backtrace:".dimmed());
            eprintln!("{:?}", e);
        }
        std::process::exit(1);
    }

    Ok(())
}
