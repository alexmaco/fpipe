use std::io::Write;
use std::io::{self, BufRead, ErrorKind};
use std::process::*;
use structopt::*;

fn main() -> Result<(), String> {
    let options = Options::from_args();

    let mut it = options.cmd_and_args.iter();
    let mut cmd = it.next().map(|name| {
        let mut cmd = Command::new(name);
        cmd.args(it).stdin(Stdio::piped());
        if options.silence {
            cmd.stdout(Stdio::null());
        }
        cmd
    });

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                return Err(format!("Error reading from stdin: {}", e));
            }
        };

        if let Some(cmd) = cmd.as_mut() {
            match run_cmd(&line, cmd) {
                Ok(false) => continue,
                Ok(true) => {}
                Err(e) => {
                    return Err(format!("Error executing command: {}", e));
                }
            }
        }
        println!("{}", line);
    }

    Ok(())
}

fn run_cmd(input: &str, cmd: &mut Command) -> io::Result<bool> {
    let mut child = cmd.spawn()?;

    let write_res = child
        .stdin
        .as_mut()
        .ok_or(io::Error::from(ErrorKind::BrokenPipe))?
        .write_all(input.as_bytes());

    // This is a race that is problematic to test.
    // The child process, at this point may either:
    //  - have exited early, of its own accord
    //  - have crashed
    //  - not have stdin open
    //  - have read some input, and exited before reading all of it
    //
    // After some manual testing, it seems safe to ignore BrokenPipe when it happens
    // The final result will still be dictated by the child exit status
    match write_res {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::BrokenPipe => {}
        Err(e) => return Err(e),
    }

    let status = child.wait()?;
    Ok(status.success())
}

#[derive(StructOpt, Debug)]
#[structopt(about = "filter (and map) items in a pipe")]
#[structopt(settings = &[clap::AppSettings::TrailingVarArg])]
struct Options {
    #[structopt(
        short,
        long,
        help = "suppress stdout of command (stderr is still propagated)"
    )]
    silence: bool,

    #[structopt(required = false, help = "command to execute and its arguments")]
    cmd_and_args: Vec<String>,
}
