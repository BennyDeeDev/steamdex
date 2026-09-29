use iced::{Element, widget::text};

fn main() -> iced::Result {
    println!("Hello World");

    iced::run(update, view)
}

fn update(_: &mut (), _: ()) {}

fn view(_: &()) -> Element<'_, ()> {
    text("Hello World!").into()
}
