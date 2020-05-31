use assert_cli::Assert;
use test_dir::TempDir;

#[test]
fn noargs() {
    assert("", &[]).stdout().is("").stderr().is("").unwrap();
}

#[test]
fn noargs_echo() {
    assert("abc\ndef", &[])
        .stdout()
        .is("abc\ndef\n")
        .stderr()
        .is("")
        .unwrap();
}

#[test]
fn no_output_when_command_fails() {
    assert("abc\ndef", &["false"])
        .stdout()
        .is("")
        .stderr()
        .is("")
        .unwrap();
}

#[test]
fn keep_input_when_command_succeeds() {
    assert("abc\ndef\n", &["true"])
        .stdout()
        .is("abc\ndef\n")
        .stderr()
        .is("")
        .unwrap();
}

#[test]
fn no_output_when_command_succeeds_but_is_negated() {
    assert("abc\ndef", &["-n", "true"])
        .stdout()
        .is("")
        .stderr()
        .is("")
        .unwrap();
}

#[test]
fn keep_input_when_command_fails_but_is_negated() {
    assert("abc\ndef\n", &["-n", "false"])
        .stdout()
        .is("abc\ndef\n")
        .stderr()
        .is("")
        .unwrap();
}

#[test]
fn each_line_gets_passed_to_command() {
    let tmp = TempDir::temp().expect("cannot create temporary directory");
    let log_file = tmp.path().join("log");
    let log_file_str = log_file
        .as_os_str()
        .to_str()
        .expect("unexpected non-utf8 in path");

    assert("abc\ndef\n", &["tee", "-a", log_file_str]).unwrap();
    assert_eq!("abcdef", std::fs::read_to_string(log_file).unwrap());
}

#[test]
fn stdout_propagates_by_default() {
    assert("abc\ndef\n", &["echo", "x"])
        .stdout()
        .is("x\nabc\nx\ndef\n")
        .stderr()
        .is("")
        .unwrap();
}

#[test]
fn stdout_not_propagated_silence_short() {
    stdout_not_propagated("-q");
}

#[test]
fn stdout_not_propagated_silence_long() {
    stdout_not_propagated("--quiet");
}

fn stdout_not_propagated(silence: &str) {
    assert("abc\ndef\n", &[silence, "echo", "x"])
        .stdout()
        .is("abc\ndef\n")
        .stderr()
        .is("")
        .unwrap();
}

#[test]
fn only_stdout_propagated_map_short() {
    basic_map("-m");
}

#[test]
fn only_stdout_propagated_map_long() {
    basic_map("--map");
}

fn basic_map(map: &str) {
    assert("abc\ndef\n", &[map, "echo", "x"])
        .stdout()
        .is("x\nx\n")
        .stderr()
        .is("")
        .unwrap();
}

#[test]
fn input_passed_to_command_when_braces_exist() {
    assert("abc\ndef\n", &["echo", "{}"])
        .stdout()
        .is("abc\nabc\ndef\ndef\n")
        .stderr()
        .is("")
        .unwrap();
}

#[test]
fn input_not_on_stdin_when_braces_exist() {
    let tmp = TempDir::temp().expect("cannot create temporary directory");
    let log_file = tmp.path().join("log");
    let log_file_str = log_file
        .as_os_str()
        .to_str()
        .expect("unexpected non-utf8 in path");

    assert(log_file_str, &["tee", "{}"]).unwrap();
    assert_eq!("", std::fs::read_to_string(log_file).unwrap());
}

#[test]
fn sigpipe_from_output_does_not_trigger_error() {
    use std::iter::repeat_with;
    let input: String = {
        let mut count = 0;
        repeat_with(|| {
            count += 1;
            count.to_string() + "\n"
        })
        .take(10000)
        .collect()
    };
    let expected: String = {
        let mut count = 0;
        repeat_with(|| {
            count += 1;
            count.to_string() + "\n"
        })
        .take(10)
        .collect()
    };

    // FIXME: maybe pipe processes together
    Assert::command(&[
        "/bin/sh",
        "-c",
        concat!(env!("CARGO_BIN_EXE_fpipe"), " | head -n 10"),
    ])
    .stdin(input)
    .stdout()
    .is(expected.as_str())
    .stderr()
    .is("")
    .unwrap();
}

fn assert(input: &str, args: &[&str]) -> Assert {
    assert_cli::Assert::command(&[env!("CARGO_BIN_EXE_fpipe")])
        .with_args(args)
        .stdin(input)
}
