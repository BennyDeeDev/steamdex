mod cli;
mod config;
mod parser;
mod steam;
mod steamgriddb;
mod vdf;

#[tokio::main]
async fn main() {
    cli::run().await;
}
