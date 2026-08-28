use std::process::ExitCode;

fn main() -> ExitCode {
    match skill_autoresearch::run_cli(std::env::args().skip(1)) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("skill-autoresearch: {error}");
            ExitCode::from(1)
        }
    }
}
