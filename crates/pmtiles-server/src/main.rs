use crate::server::serve;
use anyhow::Error;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "false")]
    serve: bool,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    serve(args.serve).await
}

mod config;
mod error;
mod routes;
mod server;
mod style;
mod utils;
