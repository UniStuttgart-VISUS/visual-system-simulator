#[macro_use]
extern crate rust_i18n;

i18n!("locales", fallback = "en");

mod cmd;
mod flow;
mod node;
mod ui;

fn main() -> Result<(), String> {
    cmd::run()
}
