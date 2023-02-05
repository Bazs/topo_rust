use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Path to the input config file.
    #[arg(short, long)]
    config_filepath: String,
}

fn main() {
    let args = Args::parse();
    dbg!(&args);
}
