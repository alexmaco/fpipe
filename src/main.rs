use std::io::ErrorKind;
use std::process::{Output, Stdio};
use std::sync::Arc;
use structopt::*;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::runtime;

fn main() -> Result<(), String> {
    let options = Arc::new(Options::from_args());

    let mut rt = runtime::Builder::new()
        .basic_scheduler()
        .enable_io()
        .build()
        .map_err(|e| format!("Failed to create runtime: {:?}", e))?;

    rt.block_on(async {
        let stdin = io::stdin();
        let buf_read = io::BufReader::new(stdin);
        let mut lines = buf_read.lines();
        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| format!("Error reading from stdin: {:?}", e))?
        {
            let options = options.clone();
            let cmd_name = options.cmd_and_args.iter().next();

            let out_buf = if let Some(cmd_name) = cmd_name {
                let mut cmd = Command::new(cmd_name);
                cmd.args(substitute_cmd_args(&line, &options))
                    .stdin(Stdio::piped());

                if options.quiet {
                    cmd.stdout(Stdio::null());
                }

                if options.map {
                    cmd.stdout(Stdio::piped());
                }

                match run_cmd(&line, &mut cmd)
                    .await
                    .map(|out| (out.status.success() ^ options.negate, out.stdout))
                {
                    Ok((false, _)) => return Ok(()),
                    Ok((true, out_buf)) => {
                        if options.map {
                            Some(out_buf)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        return Err(format!("Error executing command: {}", e));
                    }
                }
            } else {
                None
            };

            match write_out(
                out_buf.as_deref().unwrap_or_else(|| line.as_bytes()),
                out_buf.is_none(),
            )
            .await
            {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                    // output died
                    std::process::exit(0);
                }
                Err(e) => return Err(format!("Error printing output: {}", e)),
            };
        }

        Ok(())
    })
}

async fn write_out(data: &[u8], newline: bool) -> io::Result<()> {
    let mut out = io::stdout();
    out.write_all(data).await?;
    if newline {
        out.write_all(b"\n").await?;
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

/// Result indicates that subprocess ended with success status
async fn run_cmd(input: &str, cmd: &mut Command) -> io::Result<Output> {
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
        .write_all(input.as_bytes())
        .await;

    unwrap_ignore_sigpipe!(write_res);

    child.wait_with_output().await
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
