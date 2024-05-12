use clap::Parser;
use clap_stdin::FileOrStdin;

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

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    if pattern.chars().count() == 1 {
        return input_line.contains(pattern);
    } else {
        panic!("Unhandled pattern: {}", pattern)
    }
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

    let pattern = cli.pattern;

    if match_pattern(&content, &pattern) {
        Ok(())
    } else {
        anyhow::bail!("Error matching pattern")
    }
}
