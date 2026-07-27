#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

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
