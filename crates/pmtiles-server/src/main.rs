use crate::server::serve;
use anyhow::Error;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "false")]
    serve: bool,
    #[arg(short, long, default_value = "5000")]
    port: u32,
    #[arg(short, long, default_value = "127.0.0.1")]
    listen_addr: String
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    serve(args.serve, &args.listen_addr, args.port).await
}

mod config;
mod error;
mod font;
mod routes;
mod server;
mod style;
mod utils;
