use clap::{AppSettings, Parser};
use std::io::ErrorKind;
use std::process::{Output, Stdio};
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, Stdout};
use tokio::process::Command;
use tokio::runtime;

fn main() -> Result<(), String> {
    let options = Arc::new(Options::parse());

    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .map_err(|e| format!("Failed to create runtime: {:?}", e))?;

    rt.block_on(async {
        let mut out = io::stdout();
        let stdin = io::stdin();
        let buf_read = io::BufReader::new(stdin);
        let mut lines = buf_read.lines();
        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| format!("Error reading from stdin: {:?}", e))?
        {
            let out_buf = if let Some(cmd_name) = options.cmd_and_args.get(0) {
                let executing = cmd_name == "{}";

                match run_cmd(&line, cmd_name, &options)
                    .await
                    .transpose()
                    .map(|out| out.map(|out| (out.status.success() ^ options.negate, out.stdout)))
                {
                    None => None,
                    Some(Ok((false, _))) => continue,
                    Some(Ok((true, out_buf))) => {
                        if options.map {
                            Some(out_buf)
                        } else {
                            None
                        }
                    }
                    Some(Err(e)) => {
                        let err = format!("Error executing command: {}", e);
                        if executing {
                            eprintln!("{}", err);
                            continue;
                        } else {
                            return Err(err);
                        }
                    }
                }
            } else {
                None
            };

            match write_out(
                &mut out,
                out_buf.as_deref().unwrap_or_else(|| line.as_bytes()),
                out_buf.is_none(),
            )
            .await
            {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                    // output died
                    // flush and exit, we don't need the dtors
                    let _ = out.flush().await;
                    std::process::exit(0);
                }
                Err(e) => return Err(format!("Error printing output: {}", e)),
            };
        }

        Ok(())
    })
}

async fn write_out(out: &mut Stdout, data: &[u8], newline: bool) -> io::Result<()> {
    out.write_all(data).await?;
    if newline {
        out.write_all(b"\n").await?;
    }
    out.flush().await
}

fn substitute_cmd_args<'a>(
    line: &'a str,
    options: &'a Options,
) -> (Option<&'a str>, impl Iterator<Item = &'a str> + 'a) {
    let input = if options.cmd_and_args.iter().any(|s| s == "{}") {
        None
    } else {
        Some(line)
    };

    let args = options
        .cmd_and_args
        .iter()
        .skip(1)
        .map(move |arg| if arg == "{}" { line } else { arg });

    (input, args)
}

async fn run_cmd(line: &str, cmd_name: &str, options: &Options) -> io::Result<Option<Output>> {
    let executing = cmd_name == "{}";
    let mut cmd = if executing {
        let mut bits = line.split_whitespace();
        let first = match bits.next() {
            Some(b) => b,
            _ => return Ok(None),
        };
        let mut cmd = Command::new(first);
        cmd.args(bits);
        cmd
    } else {
        Command::new(cmd_name)
    };

    let (input, args) = substitute_cmd_args(line, options);
    cmd.args(args);
    if input.is_some() {
        cmd.stdin(Stdio::piped());
    }

    if options.quiet {
        cmd.stdout(Stdio::null());
    }

    if options.map {
        cmd.stdout(Stdio::piped());
    }

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
    }

    let mut child = cmd.spawn()?;

    if let Some(input) = input {
        let write_res = child
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::from(ErrorKind::BrokenPipe))?
            .write_all(input.as_bytes())
            .await;

        unwrap_ignore_sigpipe!(write_res);
    }

    child.wait_with_output().await.map(Some)
}

#[derive(Parser)]
#[clap(
    about = "
Filter (and map) in a shell pipe\n\
'{}' arguments to the command are replaced with input line before execution
",
    version
)]
#[clap(global_setting(AppSettings::TrailingVarArg))]
struct Options {
    #[clap(
        short,
        long,
        help = "Suppress stdout of command (stderr is still propagated)"
    )]
    quiet: bool,

    #[clap(short, long, help = "Negate the command exit status")]
    negate: bool,

    #[clap(
        short,
        long,
        help = "Perform mapping (only command output is emitted, only if successful)"
    )]
    map: bool,

    #[clap(required = false, help = "Command to execute and its arguments")]
    cmd_and_args: Vec<String>,
}
