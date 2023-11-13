use std::{process::Command, process::exit};

fn main() {
    if !run_command("cargo", &["fmt", "--", "--check"]) {
        exit(1);
    }

    if !run_command("cargo", &["test"]) {
        exit(1);
    }
}

fn run_command<I, S>(command: &str, args: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new(command)
        .args(args)
        .output()
        .expect("failed to run the command");

    output.status.success()
}
