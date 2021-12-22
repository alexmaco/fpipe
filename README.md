# fpipe

Filter (and map) in a shell pipe.

## What and Why

For the times you need to filter (or map) in a shell pipe.

Filtering (default):
- reads stdin as lines
- runs command for each line
- prints line if command was successful

Mapping (with `--map`):
- reads stdin as lines
- runs command for each line
- if command was successful, output its stdout instead of the line

## Examples

Only list files that contain cats:

```bash
ls | fpipe grep -sqi cats {}
```

Only keep files that do **not** contain cats:

```bash
ls | fpipe -n grep -sqi cats {} # short for --negate
```

Search for files that contain a list of patterns in the name:

```bash
cat patterns | fpipe -m fd {} # short for --map
```

## Command syntax

If `{}` is not present in the command arguments, the line is passed to the subprocess via stdin.

If `{}` is present, it gets replaced by each input line before execution.

## Installation

```bash
cargo install fpipe
```

## Flags and features

```
fpipe 0.1.0

Filter (and map) in a shell pipe
'{}' arguments to the command are replaced with input line before execution

USAGE:
    fpipe [FLAGS] [cmd-and-args]...

FLAGS:
    -h, --help       Prints help information
    -m, --map        perform mapping (only command output is emitted, only if successful)
    -n, --negate     negate the command exit status
    -q, --quiet      suppress stdout of command (stderr is still propagated)
    -V, --version    Prints version information

ARGS:
    <cmd-and-args>...    command to execute and its arguments
```

## TODO
- more features
- parallelism
