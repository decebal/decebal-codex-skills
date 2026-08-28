use std::process::ExitCode;

fn main() -> ExitCode {
    match aso_lint::run_cli(std::env::args().skip(1)) {
        Ok(outcome) => {
            println!("{}", outcome.output);
            ExitCode::from(outcome.exit_code)
        }
        Err(error) => {
            eprintln!("aso-lint: {error}");
            ExitCode::from(1)
        }
    }
}
