use std::path::PathBuf;

use tokio::fs;

mod cli;
mod config;
mod parser;
pub mod steamgriddb;
mod vdf;

#[tokio::main]
async fn main() {
    let path = PathBuf::from("test-data/shortcuts.vdf");
    vdf::read_vdf(&path);
    cli::run().await;
}
