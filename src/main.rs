use std::path::PathBuf;

mod cli;
mod config;
mod parser;
mod steam;
mod steamgriddb;
mod vdf;

#[tokio::main]
async fn main() {
    let path = PathBuf::from("test-data/shortcuts.vdf");
    cli::run().await;
}
