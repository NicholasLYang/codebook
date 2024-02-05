mod config;

use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};
use ignore::Walk;
use markdown::mdast::Node;
use markdown::ParseOptions;
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
    Check { file: Utf8PathBuf },
}

#[derive(Debug)]
enum Edit {
    Insert(String),
    Delete(String),
    Keep(String),
}

#[derive(Debug)]
enum Snippet {
    // Inserts the content at the given line in the file
    InsertAt {
        file: Utf8PathBuf,
        line: usize,
        edits: Vec<Edit>,
    },
    Create {
        path: Utf8PathBuf,
        content: String,
    },
    Delete {
        path: Utf8PathBuf,
    },
}

impl Snippet {
    fn parse_content_for_insert(content: &str) -> Vec<Edit> {
        let mut has_no_inserts_or_deletes = true;
        let edits = content
            .lines()
            .map(|line| {
                if let Some(line) = line.strip_prefix("+ ") {
                    has_no_inserts_or_deletes = false;
                    Edit::Insert(line.to_string())
                } else if let Some(line) = line.strip_prefix("- ") {
                    has_no_inserts_or_deletes = false;
                    Edit::Delete(line.to_string())
                } else {
                    Edit::Keep(line.to_string())
                }
            })
            .collect();

        if has_no_inserts_or_deletes {
            println!("warning: no inserts or deletes found in content")
        }

        edits
    }
    fn parse(metadata: &str, content: &str) -> Result<Snippet, anyhow::Error> {
        let metadata = metadata.trim();
        let (action, s) = metadata.split_once(':').ok_or(anyhow!(
            "expected location with format <file>@<line> instead received {}",
            metadata
        ))?;

        match action {
            "insert" => {
                let (file, line) = s.split_once('@').ok_or(anyhow!(
                    "expected location with format <file>@<line> instead received {}",
                    s
                ))?;
                let line = line.parse::<usize>()?;
                let edits = Snippet::parse_content_for_insert(content);

                Ok(Snippet::InsertAt {
                    file: Utf8PathBuf::from(file.trim()),
                    edits,
                    line,
                })
            }
            "create" => Ok(Snippet::Create {
                path: Utf8PathBuf::from(s.trim()),
                content: content.to_string(),
            }),
            "delete" => {
                if !content.is_empty() {
                    println!("warning: content will be ignored for delete action")
                }
                Ok(Snippet::Delete {
                    path: Utf8PathBuf::from(s.trim()),
                })
            }
            _ => Err(anyhow!("unknown action {}", action)),
        }
    }
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    match args.command {
        Command::Check { file } => {
            let cwd = if let Some(cwd) = args.cwd {
                cwd
            } else {
                Utf8PathBuf::try_from(std::env::current_dir()?)?
            };

            let file = cwd.join(file);

            let file_content = fs::read_to_string(&file)?;
            let ast = markdown::to_mdast(&file_content, &ParseOptions::default()).unwrap();
            let snippets = get_snippets(&ast)?;

            let config = config::Config::load(cwd.as_std_path())?;

            check_each_snippet(&cwd, &config.test.command, snippets)?;
        }
    }
    Ok(())
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

fn check_each_snippet(
    cwd: &Utf8Path,
    check_command: &str,
    snippets: Vec<Snippet>,
) -> Result<(), anyhow::Error> {
    // Copy everything to a temp directory to avoid side effects.
    let tempdir = tempfile::tempdir()?;
    copy_dir(cwd.as_std_path(), tempdir.path())?;

    let cwd = tempdir.path();

    let command_tokens = shlex::split(check_command).ok_or(anyhow!(
        "failed to parse command {} into tokens",
        check_command
    ))?;
    let mut command = std::process::Command::new(&command_tokens[0]);
    command.args(&command_tokens[1..]).current_dir(cwd);

    for (idx, snippet) in snippets.into_iter().enumerate() {
        match snippet {
            Snippet::InsertAt { file, line, edits } => {
                let file = cwd.join(file);

                let content = fs::read_to_string(&file)?;
                let mut lines: Vec<&str> = content.lines().collect();
                let mut idx = line;
                for edit in &edits {
                    match edit {
                        Edit::Insert(line) => {
                            lines.insert(idx, line);
                            idx += 1;
                        }
                        Edit::Delete(content) => {
                            if lines.get(idx) != Some(&(content.as_str())) {
                                return Err(anyhow!(
                                    "expected line to delete {} but found {}",
                                    &content[1..],
                                    lines.get(idx).unwrap()
                                ));
                            }
                            lines.remove(idx);
                        }
                        Edit::Keep(content) => {
                            if lines.get(idx) != Some(&(content.as_str())) {
                                return Err(anyhow!(
                                    "expected line to keep {} but found {}",
                                    &content,
                                    lines.get(idx).unwrap()
                                ));
                            }
                            idx += 1;
                        }
                    }
                }

                fs::write(&file, lines.join("\n"))?;
            }
            Snippet::Create { path, content } => {
                fs::write(cwd.join(&path), content)?;
            }
            Snippet::Delete { path } => {
                fs::remove_file(cwd.join(&path))?;
            }
        }

        let output = command.output()?;
        if !output.status.success() {
            println!("Snippet #{} failed", idx);
            println!(
                "{}",
                anyhow!(
                    "command failed with status {}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                )
            );
        } else {
            println!("snippet #{} passed", idx);
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }

    Ok(())
}

fn add_snippets(snippets: &mut Vec<Snippet>, node: &Node) -> Result<(), anyhow::Error> {
    match node {
        Node::Code(code) => {
            if let Some(metadata) = &code.meta {
                let snippet = Snippet::parse(metadata, &code.value)?;
                snippets.push(snippet);
            }
        }
        node => {
            for child in node.children().into_iter().flatten() {
                add_snippets(snippets, child)?;
            }
        }
    }

    Ok(())
}

fn get_snippets(node: &Node) -> Result<Vec<Snippet>, anyhow::Error> {
    let mut snippets = Vec::new();
    add_snippets(&mut snippets, node)?;

    Ok(snippets)
}
