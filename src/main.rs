mod cli;
mod error;
mod git;
mod graph;
mod i18n;
mod insights;
mod refs;
mod relation;
mod render;
mod report;

fn main() {
    let language = i18n::Language::load();
    let cli = cli::parse(language);
    if let Err(error) = report::run(cli, language) {
        eprintln!("{}", error.localized(language));
        std::process::exit(error.exit_code());
    }
}
