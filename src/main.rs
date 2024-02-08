mod config;
mod snippet;

use crate::config::Config;
use anyhow::{anyhow, Context};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};
use clean_path::clean;
use colored::Colorize;
use dialoguer::FuzzySelect;
use ignore::Walk;
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long)]
    cwd: Option<Utf8PathBuf>,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    Check,
    Generate {
        out_dir: Utf8PathBuf,
        file: Option<Utf8PathBuf>,
        snippet_idx: Option<usize>,
    },
}

#[derive(Clone, Debug)]
enum EditLine {
    Add(String),
    Delete(String),
    Keep(String),
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    let cwd = if let Some(cwd) = args.cwd {
        cwd
    } else {
        Utf8PathBuf::try_from(std::env::current_dir()?)?
    };

    match args.command {
        Command::Check => {
            let config = Config::load(cwd.as_std_path()).context("could not load codebook.toml")?;
            for file in config.files {
                let file = cwd.join(file);
                let snippets = snippet::get_snippets(&file)?;

                snippet::check_each_snippet(
                    &cwd,
                    config.test.as_ref().and_then(|t| t.command.as_deref()),
                    snippets,
                )?;
            }
        }
        Command::Generate {
            out_dir,
            file,
            snippet_idx,
        } => {
            fs::create_dir_all(&out_dir)?;
            copy_dir(cwd.as_std_path(), out_dir.as_std_path())?;

            let config = Config::load(cwd.as_std_path())?;
            let (file, idx) = if let Some(file) = file.as_deref() {
                let idx = config.get_file_idx(&file)?;
                (file, idx)
            } else {
                config.select_file()?
            };

            let files = &config.files[..idx];

            let file = cwd.join(file);
            let snippets = snippet::get_snippets(&file)?;
            let snippet_idx = if let Some(idx) = snippet_idx {
                idx
            } else {
                FuzzySelect::new()
                    .with_prompt("Select snippet")
                    .items(&snippets)
                    .default(0)
                    .interact()?
            };

            // First apply the files before the selected file.
            for file in files {
                let file = cwd.join(file);
                let snippets = snippet::get_snippets(&file)?;
                snippet::apply_snippets(out_dir.as_std_path(), snippets, |idx| {
                    println!("applying snippet #{} in {}", idx, file);
                    Ok(())
                })?;
            }

            // Then apply the snippets for the selected file.
            snippet::apply_snippets(
                out_dir.as_std_path(),
                snippets[..=snippet_idx].to_vec(),
                |idx| {
                    println!(
                        "{} in {}",
                        format!("applying snippet #{}", idx).blue().bold(),
                        file
                    );
                    Ok(())
                },
            )?;
        }
    }
    Ok(())
}

impl Config {
    fn select_file(&self) -> Result<(&Utf8Path, usize), anyhow::Error> {
        let idx = FuzzySelect::new()
            .with_prompt("Select file")
            .items(&self.files)
            .default(0)
            .interact()?;

        Ok((&self.files[idx], idx))
    }

    fn get_file_idx(&self, file: &Utf8Path) -> Result<usize, anyhow::Error> {
        let file = clean(file);

        self.files
            .iter()
            .position(|f| f.as_std_path() == file)
            .ok_or(anyhow!("file not found in codebook.toml"))
    }
}

fn copy_dir(src: &Path, dest: &Path) -> Result<(), anyhow::Error> {
    fs::create_dir_all(dest)?;
    for entry in Walk::new(src) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(src)?;
        let dest = dest.join(relative);
        if entry.metadata()?.is_dir() {
            fs::create_dir_all(&dest)?;
        } else {
            fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}
