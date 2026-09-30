use clap::{Parser, Subcommand};
use iced::{Element, widget::text};

#[derive(Parser)]
#[command(
    name = "lududex",
    about = "A cross-platform steam library manager",
    long_about = "Discover games and ROMs with configurable parsers, then organize them in your Steam library.",
    version
)]

struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Greet {
        /// The person to greet.
        name: String,
    },
}

fn main() -> iced::Result {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Greet { name }) => {
            println!("Hello: {name}");
            Ok(())
        }
        None => iced::run(update, view),
    }
}

fn update(_: &mut (), _: ()) {}

fn view(_: &()) -> Element<'_, ()> {
    text("Hello World!").into()
}
