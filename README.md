# fpipe

Filter (and map) in a shell pipe.

## What and Why

For the times you need to filter (or map) in a shell pipe.

Filtering (default):
- reads input lines
- runs command for each line
- prints line if command was successful

Mapping (with `--map`):
- reads input lines
- runs command for each line
- if command was successful, output its stdout instead of the line

## Examples

Only list files that contain cats:

```bash
ls | fpipe grep -qi cats {}
```

Only keep files that do **not** contain cats:

```bash
ls | fpipe -n grep -qi cats {} # short for --negate
```

Search for files that contain a list of patterns in the name:

```bash
cat patterns | fpipe -m fd {} # short for --map
```

## Command syntax

If `{}` is not present in the command arguments, the line is passed to the subprocess via stdin.

If `{}` is given, it is replaced by each line in turn before execution.

## Installation

`cargo install -f --git https://github.com/alexmaco/fpipe`
