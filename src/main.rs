use std::io::{self, BufRead, ErrorKind};
use std::io::{StdoutLock, Write};
use std::process::*;
use structopt::*;

fn main() -> Result<(), String> {
    let options = Options::from_args();

    let cmd_name = options.cmd_and_args.iter().next();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                return Err(format!("Error reading from stdin: {}", e));
            }
        };

        if let Some(cmd_name) = cmd_name {
            let mut cmd = Command::new(cmd_name);
            cmd.args(substitute_cmd_args(&line, &options))
                .stdin(Stdio::piped());

            if options.silence {
                cmd.stdout(Stdio::null());
            }

            match run_cmd(&line, &mut cmd).map(|success| success ^ options.negate) {
                Ok(false) => continue,
                Ok(true) => {}
                Err(e) => {
                    return Err(format!("Error executing command: {}", e));
                }
            }
        }

        match write_out(&mut out, &line) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                // output died
                return Ok(());
            }
            Err(e) => return Err(format!("Error printing output: {}", e)),
        }
    }

    Ok(())
}

fn write_out(out: &mut StdoutLock, line: &str) -> io::Result<()> {
    out.write_all(line.as_bytes())?;
    out.write_all(b"\n")
}

fn substitute_cmd_args<'a>(
    line: &'a str,
    options: &'a Options,
) -> impl Iterator<Item = &'a str> + 'a {
    options
        .cmd_and_args
        .iter()
        .skip(1)
        .map(move |arg| if arg == "{}" { line } else { &arg })
}

fn run_cmd(input: &str, cmd: &mut Command) -> io::Result<bool> {
    let mut child = cmd.spawn()?;

    let write_res = child
        .stdin
        .as_mut()
        .ok_or_else(|| io::Error::from(ErrorKind::BrokenPipe))?
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

    #[structopt(short, long, help = "negate the command exit status")]
    negate: bool,

    #[structopt(required = false, help = "command to execute and its arguments")]
    cmd_and_args: Vec<String>,
}
