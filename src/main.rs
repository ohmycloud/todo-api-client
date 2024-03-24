use clap::{Parser, Subcommand};
use colored_json::ToColoredJson;
use http_body_util::Full;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::header::CONTENT_TYPE;
use hyper::{Method, Request, Uri};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde_json::json;
use yansi::Paint;

async fn request(
    url: hyper::Uri,
    method: Method,
    body: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build_http();

    let req: Request<Full<Bytes>> = Request::builder()
        .uri(url)
        .method(&method)
        .header("Content-Type", "application/json")
        .body(Full::from(body.unwrap_or("".to_string())))
        .unwrap();

    let mut res = client.request(req).await?;

    let mut buf = Vec::new();
    while let Some(next) = res.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            buf.extend_from_slice(&chunk);
        }
    }
    let s = String::from_utf8(buf)?;
    eprintln!("Status: {}", Paint::green(&res.status()));

    if res.headers().contains_key(CONTENT_TYPE) {
        let content_type = res.headers()[CONTENT_TYPE].to_str()?;
        eprintln!("Content-Type: {}", Paint::green(content_type));

        if content_type.starts_with("application/json") {
            println!("{}", &s.to_colored_json_auto()?);
        } else {
            println!("{}", &s);
        }
    } else {
        println!("{}", &s);
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all todos
    List,
    /// Create a new todo
    Create {
        /// The todo body
        body: String,
    },
    /// Read a todo
    Read {
        /// The todo id
        id: i64,
    },
    /// Update a todo
    Update {
        /// The todo ID
        id: i64,
        /// The todo body
        body: String,
        /// Mark todo as completed
        #[arg(short, long)]
        completed: bool,
    },
    /// Delete a todo
    Delete {
        /// The todo ID
        id: i64,
    },
}

#[derive(Parser, Debug)]
struct Cli {
    /// Base URL of API service
    url: hyper::Uri,
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    // Parse the base URl into its parts, and we'll add them into a new hyper::Uri builder.
    let mut uri_builder = Uri::builder();
    if let Some(scheme) = cli.url.scheme() {
        // Extract the base URL scheme(i.e. http or https)
        uri_builder = uri_builder.scheme(scheme.clone());
    }
    if let Some(authority) = cli.url.authority() {
        // Extract the base URL authority(i.e., localhost or 127.0.0.1)
        uri_builder = uri_builder.authority(authority.clone());
    }

    match cli.command {
        Commands::List => {
            request(
                uri_builder.path_and_query("/v1/todos").build()?,
                Method::GET,
                None,
            )
            .await?
        }
        Commands::Delete { id } => {
            request(
                uri_builder
                    .path_and_query(format!("/v1/todos/{}", id))
                    .build()?,
                Method::DELETE,
                None,
            )
            .await?
        }
        Commands::Read { id } => {
            request(
                uri_builder
                    .path_and_query(format!("/v1/todos/{}", id))
                    .build()?,
                Method::GET,
                None,
            )
            .await?
        }
        Commands::Create { body } => {
            request(
                uri_builder.path_and_query("/v1/todos").build()?,
                Method::POST,
                Some(json!({ "body": body }).to_string()),
            )
            .await?
        }
        Commands::Update {
            id,
            body,
            completed,
        } => {
            request(
                uri_builder
                    .path_and_query(format!("/v1/todos/{}", id))
                    .build()?,
                Method::PUT,
                Some(json!({"body":body,"completed":completed}).to_string()),
            )
            .await?
        }
    }

    Ok(())
}
