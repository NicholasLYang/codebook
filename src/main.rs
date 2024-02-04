use camino::Utf8PathBuf;
use clap::Parser;
use markdown::ParseOptions;

#[derive(Parser, Debug)]
struct Args {
    file: Utf8PathBuf
}
fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    let file_content = std::fs::read_to_string(&args.file)?;
    let ast = markdown::to_mdast(&file_content, &ParseOptions::default()).unwrap();
    println!("{:#?}", ast);

    Ok(())
}
