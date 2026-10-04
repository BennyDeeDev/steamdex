mod cli;
mod config;
mod parser;
pub mod steamgriddb;

#[tokio::main]
async fn main() {
    cli::run().await;
}
