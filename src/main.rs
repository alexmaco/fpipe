use clap::{AppSettings, Parser};
use std::io::ErrorKind;
use std::process::{Output, Stdio};
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, Stdout};
use tokio::process::Command;
use tokio::runtime;

struct ExecInfo {
    command_from_imput: bool,
    subprocess_takes_input: bool,
}

fn main() -> Result<(), String> {
    let options = Arc::new(Options::parse());

    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .map_err(|e| format!("Failed to create runtime: {:?}", e))?;

    let exec_info = ExecInfo {
        command_from_imput: options
            .cmd_and_args
            .get(0)
            .map_or(false, |name| name == "{}"),
        subprocess_takes_input: !options.cmd_and_args.iter().any(|s| s == "{}"),
    };

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
                match run_cmd(&line, cmd_name, &options, &exec_info).await {
                    Ok(None) => None,
                    Ok(Some(res)) => {
                        let succeded = res.status.success() ^ options.negate;
                        if !succeded {
                            continue;
                        } else if options.map {
                            Some(res.stdout)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        let err = format!("Error executing command: {}", e);
                        if exec_info.command_from_imput {
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

            let write_result = match out_buf {
                Some(data) => write_out(&mut out, &data, false).await,
                None => write_out(&mut out, line.as_bytes(), true).await,
            };

            match write_result {
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

        // flush and exit, we don't need the dtors
        let _ = out.flush().await;
        std::process::exit(0);
    })
}

async fn write_out(out: &mut Stdout, data: &[u8], newline: bool) -> io::Result<()> {
    out.write_all(data).await?;
    if newline {
        out.write_all(b"\n").await?;
    }
    out.flush().await
}

async fn run_cmd(
    line: &str,
    cmd_name: &str,
    options: &Options,
    exec_info: &ExecInfo,
) -> io::Result<Option<Output>> {
    let mut cmd = if exec_info.command_from_imput {
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

    cmd.args(
        options
            .cmd_and_args
            .iter()
            .skip(1)
            .map(|arg| if arg == "{}" { line } else { arg }),
    );

    if exec_info.subprocess_takes_input {
        cmd.stdin(Stdio::piped());
    }

    if options.quiet {
        cmd.stdout(Stdio::null());
    }

    if options.map {
        cmd.stdout(Stdio::piped());
    }

    let mut child = cmd.spawn()?;

    if exec_info.subprocess_takes_input {
        let write_res = child
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::from(ErrorKind::BrokenPipe))?
            .write_all(line.as_bytes())
            .await;

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
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::BrokenPipe => {}
            Err(e) => return Err(e),
        }
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
