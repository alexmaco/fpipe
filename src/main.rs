use std::io::Read;
use std::io::{self, BufRead, ErrorKind};
use std::io::{StdoutLock, Write};
use std::process::*;
use structopt::*;

fn main() -> Result<(), String> {
    let options = Options::from_args();

    let cmd_name = options.cmd_and_args.iter().next();

    let mut out_buf = if options.map {
        Some(Vec::with_capacity(4096))
    } else {
        None
    };

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

            if options.quiet {
                cmd.stdout(Stdio::null());
            }

            if let Some(out_buf) = &mut out_buf {
                out_buf.clear();
                cmd.stdout(Stdio::piped());
            }

            match run_cmd(&line, &mut cmd, out_buf.as_mut()).map(|success| success ^ options.negate)
            {
                Ok(false) => continue,
                Ok(true) => {}
                Err(e) => {
                    return Err(format!("Error executing command: {}", e));
                }
            }
        }

        match write_out(
            &mut out,
            out_buf.as_deref().unwrap_or_else(|| line.as_bytes()),
            out_buf.is_none(),
        ) {
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

fn write_out(out: &mut StdoutLock, data: &[u8], newline: bool) -> io::Result<()> {
    out.write_all(data)?;
    if newline {
        out.write_all(b"\n")?;
    }
    Ok(())
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

fn run_cmd(input: &str, cmd: &mut Command, out_buf: Option<&mut Vec<u8>>) -> io::Result<bool> {
    macro_rules! unwrap_ignore_sigpipe {
        ($res:expr) => {
            // This is a race that is problematic to test.
            // The child process, at this point may either:
            //  - have exited early, of its own accord
            //  - have crashed
            //  - not have stdin open
            //  - have read some input, and exited before reading all of it
            //
            // After some manual testing, it seems safe to ignore BrokenPipe when it happens
            // The final result will still be dictated by the child exit status
            match $res {
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::BrokenPipe => {}
                Err(e) => return Err(e),
            }
        };
    };

    let mut child = cmd.spawn()?;

    let write_res = child
        .stdin
        .as_mut()
        .ok_or_else(|| io::Error::from(ErrorKind::BrokenPipe))?
        .write_all(input.as_bytes());

    unwrap_ignore_sigpipe!(write_res);

    //FIXME: should probably switch to async to not break for large chunks
    if let Some(out_buf) = out_buf {
        let read_res = child
            .stdout
            .as_mut()
            .ok_or_else(|| io::Error::from(ErrorKind::BrokenPipe))?
            .read_to_end(out_buf);

        unwrap_ignore_sigpipe!(read_res);
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
        help = "suppress stdout of subcommand (stderr is still propagated)"
    )]
    quiet: bool,

    #[structopt(short, long, help = "negate the command exit status")]
    negate: bool,

    #[structopt(
        short,
        long,
        help = "perform mapping (only command output is emitted, only if successful)"
    )]
    map: bool,

    #[structopt(required = false, help = "command to execute and its arguments")]
    cmd_and_args: Vec<String>,
}
