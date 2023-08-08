mod cli;
mod config;
mod expand;
mod help;
mod locate;
mod shell;
mod tokenizer;
mod tty;
mod update;
mod verbosity;

fn main() {
    let exit_code = cli::main();
    std::process::exit(exit_code);
}
