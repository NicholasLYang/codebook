use crate::EditLine;
use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};
use colored::Colorize;
use markdown::mdast::Node;
use markdown::ParseOptions;
use prettydiff::diff_chars;
use std::fmt::Display;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub enum Snippet {
    // Inserts the content at the given line in the file
    Edit {
        file: Utf8PathBuf,
        line: usize,
        edit_lines: Vec<EditLine>,
    },
    Create {
        path: Utf8PathBuf,
        content: String,
    },
    Delete {
        path: Utf8PathBuf,
    },
}

impl Display for Snippet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Snippet::Edit { file, line, .. } => {
                write!(f, "edit: {}@{}", file, line)?;
            }
            Snippet::Create { path, .. } => {
                write!(f, "create: {}", path)?;
            }
            Snippet::Delete { path } => {
                write!(f, "delete: {}", path)?;
            }
        }

        Ok(())
    }
}

impl Snippet {
    fn parse_edit_content(content: &str) -> Vec<EditLine> {
        let mut has_no_inserts_or_deletes = true;
        let edits = content
            .lines()
            .map(|line| {
                if let Some(line) = line.strip_prefix("+ ") {
                    has_no_inserts_or_deletes = false;
                    EditLine::Add(line.to_string())
                } else if let Some(line) = line.strip_prefix("- ") {
                    has_no_inserts_or_deletes = false;
                    EditLine::Delete(line.to_string())
                } else {
                    EditLine::Keep(line.to_string())
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
            "edit" => {
                let (file, line) = s.split_once('@').ok_or(anyhow!(
                    "expected location with format <file>@<line> instead received {}",
                    s
                ))?;
                let line = line.parse::<usize>()?;
                let edits = Snippet::parse_edit_content(content);

                Ok(Snippet::Edit {
                    file: Utf8PathBuf::from(file.trim()),
                    edit_lines: edits,
                    line,
                })
            }
            "create" => Ok(Snippet::Create {
                path: Utf8PathBuf::from(s.trim()),
                content: content.to_string(),
            }),
            "delete" => Ok(Snippet::Delete {
                path: Utf8PathBuf::from(s.trim()),
            }),
            _ => Err(anyhow!("unknown action {}", action)),
        }
    }
}

pub fn check_each_snippet(
    cwd: &Utf8Path,
    test_command: Option<&str>,
    snippets: Vec<Snippet>,
) -> Result<(), anyhow::Error> {
    // Copy everything to a temp directory to avoid side effects.
    let tempdir = tempfile::tempdir()?;
    let dir = tempdir.path();
    crate::copy_dir(cwd.as_std_path(), dir)?;

    let mut command = if let Some(command) = test_command {
        let command_tokens = shlex::split(command)
            .ok_or(anyhow!("failed to parse command {} into tokens", command))?;
        let mut command = std::process::Command::new(&command_tokens[0]);
        command.args(&command_tokens[1..]).current_dir(dir);

        Some(command)
    } else {
        None
    };

    apply_snippets(tempdir.path(), snippets, |idx| {
        if let Some(command) = &mut command {
            let output = command.output()?;
            if !output.status.success() {
                println!("{}", format!("Snippet #{} failed", idx).red().bold());
                println!(
                    "{}",
                    anyhow!(
                        "command failed with status {}: {}",
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    )
                );
            } else {
                println!("{}", format!("snippet #{} passed", idx).blue().bold());
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
        } else {
            println!("{}", format!("snippet #{} passed", idx).blue().bold());
        }

        Ok(())
    })
}

/// Apply the snippets to the given directory and call the snippet_fn for each snippet.
pub fn apply_snippets(
    dir: &Path,
    snippets: Vec<Snippet>,
    mut snippet_fn: impl FnMut(usize) -> Result<(), anyhow::Error>,
) -> Result<(), anyhow::Error> {
    for (idx, snippet) in snippets.into_iter().enumerate() {
        apply_snippet(dir, snippet)?;
        snippet_fn(idx)?;
    }

    Ok(())
}

pub fn apply_snippet(dir: &Path, snippet: Snippet) -> Result<(), anyhow::Error> {
    match snippet {
        Snippet::Edit {
            file,
            line,
            edit_lines: edits,
        } => {
            let file = dir.join(file);

            let content = fs::read_to_string(&file)?;
            let mut lines: Vec<&str> = content.lines().collect();
            let mut idx = line;
            for edit in &edits {
                match edit {
                    EditLine::Add(line) => {
                        lines.insert(idx, line);
                        idx += 1;
                    }
                    EditLine::Delete(content) => {
                        let Some(line) = lines.get(idx) else {
                            return Err(anyhow!(
                                "expected line to be {} but found end of file",
                                content
                            ));
                        };

                        if *line != content.as_str() {
                            return Err(anyhow!(
                                "mismatch between line and deleted one:\n{}",
                                diff_chars(&content[1..], line)
                            ));
                        }
                        lines.remove(idx);
                    }
                    EditLine::Keep(content) => {
                        if lines.get(idx) != Some(&(content.as_str())) {
                            return Err(anyhow!(
                                "expected line to be {} but found {}",
                                &content,
                                lines.get(idx).unwrap_or(&"end of file")
                            ));
                        }
                        idx += 1;
                    }
                }
            }

            fs::write(&file, lines.join("\n"))?;
        }
        Snippet::Create { path, content } => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(dir.join(parent))?;
            }
            fs::write(dir.join(&path), content)?;
        }
        Snippet::Delete { path } => {
            fs::remove_file(dir.join(&path))?;
        }
    }

    Ok(())
}

fn add_snippets(snippets: &mut Vec<Snippet>, node: &Node) -> Result<(), anyhow::Error> {
    match node {
        Node::Code(code) => {
            if let Some(metadata) = &code.meta {
                println!("metadata: {}", metadata);
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

pub fn get_snippets(file: &Utf8Path) -> Result<Vec<Snippet>, anyhow::Error> {
    let file_content = fs::read_to_string(&file)?;
    let ast = markdown::to_mdast(&file_content, &ParseOptions::default()).unwrap();
    let mut snippets = Vec::new();
    add_snippets(&mut snippets, &ast)?;

    Ok(snippets)
}
