//! CLI поверх [`blog_client::BlogClient`]; после входа/регистрации сохраняет JWT в `.blog_token`.

use std::fs;
use std::path::Path;

use anyhow::Context;
use blog_client::{BlogClient, Transport};
use clap::{Parser, Subcommand};

const TOKEN_PATH: &str = ".blog_token";
const DEFAULT_HTTP: &str = "http://localhost:8080";
const DEFAULT_GRPC: &str = "http://localhost:50051";

#[derive(Parser)]
#[command(name = "blog-cli", version)]
struct Cli {
    #[arg(long, global = true, help = "Use gRPC transport instead of HTTP")]
    grpc: bool,
    #[arg(long, global = true, help = "Server URL (HTTP base or gRPC endpoint)")]
    server: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Register {
        #[arg(long)]
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
    },
    Login {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
    },
    Get {
        #[arg(long)]
        id: i64,
    },
    Update {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: Option<String>,
    },
    Delete {
        #[arg(long)]
        id: i64,
    },
    List {
        #[arg(long, default_value_t = 10)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    let addr = resolve_server(cli.grpc, cli.server.as_deref());
    let transport = if cli.grpc {
        Transport::Grpc(addr)
    } else {
        Transport::Http(addr)
    };

    let mut client = BlogClient::new(transport)
        .await
        .context("create BlogClient")?;

    if let Ok(token) = load_token() {
        if !token.is_empty() {
            client.set_token(token);
        }
    }

    match cli.command {
        Commands::Register {
            username,
            email,
            password,
        } => {
            let auth = client
                .register(&username, &email, &password)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            save_token(&auth.token).context("save token")?;
            println!("Registered as {} (token saved to {}).", username, TOKEN_PATH);
            println!("Token: {}", auth.token);
        }
        Commands::Login { username, password } => {
            let auth = client
                .login(&username, &password)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            save_token(&auth.token).context("save token")?;
            println!("Logged in as {} (token saved).", username);
            println!("Token: {}", auth.token);
        }
        Commands::Create { title, content } => {
            let post = client
                .create_post(&title, &content)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("Created post id={} title={}", post.id, post.title);
        }
        Commands::Get { id } => {
            let post = client.get_post(id).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            println!(
                "Post {} | author {} | {}\n{}",
                post.id, post.author_id, post.title, post.content
            );
            println!(
                "created_at={} updated_at={}",
                post.created_at, post.updated_at
            );
        }
        Commands::Update {
            id,
            title,
            content,
        } => {
            let content = match content {
                Some(c) => c,
                None => {
                    let existing = client.get_post(id).await.map_err(|e| anyhow::anyhow!("{}", e))?;
                    existing.content
                }
            };
            let post = client
                .update_post(id, &title, &content)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("Updated post id={} title={}", post.id, post.title);
        }
        Commands::Delete { id } => {
            client.delete_post(id).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("Deleted post {}.", id);
        }
        Commands::List { limit, offset } => {
            let list = client
                .list_posts(limit, offset)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            println!(
                "total={} limit={} offset={}",
                list.total, list.limit, list.offset
            );
            for p in list.posts {
                let preview: String = p.content.chars().take(80).collect();
                println!(
                    "[{}] {} (author {}) — {}",
                    p.id, p.title, p.author_id, preview
                );
            }
        }
    }

    Ok(())
}

fn resolve_server(grpc: bool, server: Option<&str>) -> String {
    match server {
        Some(s) => s.to_string(),
        None => {
            if grpc {
                DEFAULT_GRPC.to_string()
            } else {
                DEFAULT_HTTP.to_string()
            }
        }
    }
}

fn load_token() -> anyhow::Result<String> {
    let path = Path::new(TOKEN_PATH);
    if !path.exists() {
        anyhow::bail!("no token file");
    }
    Ok(fs::read_to_string(path)?.trim().to_string())
}

fn save_token(token: &str) -> anyhow::Result<()> {
    fs::write(TOKEN_PATH, token)?;
    Ok(())
}
