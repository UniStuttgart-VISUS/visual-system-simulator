mod render;
mod show;

use clap::{Parser, Subcommand};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use vss::*;

#[derive(Debug, Default)]
pub(super) struct CommonConfig {
    pub inputs: Vec<String>,
    pub base_config: Option<PathBuf>,
    pub config_document: ConfigDocument,
}

#[derive(Debug, Parser)]
#[command(
    name = "vss-desktop",
    version = "1.1.0",
    author = "The Visual System Simulator Developers",
    about = "Simulates various aspects of the human visual system",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Starts the interactive simulation")]
    Show(show::ShowArgs),
    #[command(about = "Renders one or more inputs")]
    Render(render::RenderArgs),
}

fn read_text_file(path: &Path) -> Result<Option<String>, Box<dyn Error>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(std::io::Error::new(
            err.kind(),
            format!("failed to read configuration {}: {err}", path.display()),
        )
        .into()),
    }
}

fn refresh_flow_configs(config: &mut CommonConfig) -> Result<Vec<Diagnostic>, Box<dyn Error>> {
    let (documents, diagnostics) = vss::load_config_layers_from_text(
        config.base_config.as_deref(),
        &config.inputs,
        read_text_file,
    )?;
    config.config_document = documents;
    Ok(diagnostics)
}

pub(super) fn report_diagnostics(diagnostics: &[Diagnostic]) -> Result<(), String> {
    let message = vss::diagnostics_to_string(diagnostics);
    if !diagnostics.is_empty() {
        eprintln!("{message}");
    }
    (!vss::diagnostics_have_errors(diagnostics))
        .then_some(())
        .ok_or(message)
}

pub(crate) fn run() -> Result<(), String> {
    let command = Cli::try_parse_from(std::env::args_os())
        .map_err(|err| err.to_string())
        .map(|cli| {
            cli.command
                .unwrap_or_else(|| Command::Show(show::ShowArgs::default()))
        })?;

    match command {
        Command::Render(args) => render::run(args),
        Command::Show(args) => show::run(args),
    }
}
