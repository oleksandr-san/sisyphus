mod db;
mod handlers;
mod models;
mod schemas;

#[cfg(test)]
mod test;

use actix_web::{App, HttpServer};
use mongodb::Client;
use structopt::StructOpt;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{filter, prelude::*, EnvFilter};

#[derive(StructOpt, Debug)]
pub struct Options {
    #[structopt(long, short)]
    pub port: Option<u16>,

    #[structopt(long, short)]
    pub host: Option<String>,
}

fn init_logger() {
    let stdout_log = tracing_subscriber::fmt::layer()
        // .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_writer(std::io::stdout)
        .with_filter(
            EnvFilter::builder()
                .with_default_directive(filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        );

    tracing_subscriber::registry()
        .with(stdout_log)
        .try_init()
        .expect("Failed to setup global logger");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();
    let options = Options::from_args();

    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());
    let client = Client::with_uri_str(uri)
        .await
        .expect("failed to connect to MongoDB");

    db::create_username_index(&client).await;
    db::create_task_indices(&client).await;

    let host = options.host.unwrap_or("127.0.0.1".to_string());
    let port = options.port.unwrap_or(8080);
    info!("Starting server at http://{}:{}", host, port);

    HttpServer::new(move || App::new().configure(handlers::configure(client.clone())))
        .bind((host, port))?
        .run()
        .await
}
