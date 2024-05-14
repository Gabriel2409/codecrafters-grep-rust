mod regex_lexer;
mod regex_matcher;
mod regex_parser;

use clap::Parser;
use clap_stdin::FileOrStdin;
use regex_lexer::RegexLexer;

use crate::regex_matcher::Matcher;
use crate::regex_parser::RegexParser;

#[derive(Parser)]
#[command(
    version,
    about = "Custom grep",
    long_about = "Search for patterns in a file"
)]
struct Cli {
    #[arg(
        short('E'),
        long,
        help = "Interpret patterns as extended regular expression",
        // required = true
    )]
    extended_regexp: bool,
    #[arg(help = "One or more patterns separated by newline characters")]
    pattern: String,
    #[arg(
        help = "A file of - stands for standard input. In this version, there is no recursive search, so no files also means standard input",
        default_value = "-"
    )]
    file: FileOrStdin<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let content = cli.file.contents()?;

    // By default, clap exits with status code 2 when we don't pass the required
    // arguments. To exit with status code 1, we need to handle it manually.
    if !cli.extended_regexp {
        println!("Expected first argument to be '-E'");
        std::process::exit(1);
    }

    let pat = cli.pattern;
    let chars = content.chars().collect::<Vec<_>>();

    let lexer = RegexLexer::new(&pat);
    let mut parser = RegexParser::new(lexer)?;

    let node = parser.build_ast(0)?;
    let mut matcher = Matcher::new(chars.len());
    let is_match = matcher.matches(&node, &chars);

    if is_match {
        Ok(())
    } else {
        anyhow::bail!("Error matching pattern")
    }
}
