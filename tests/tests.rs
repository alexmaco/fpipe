use assert_cmd::{assert::Assert, Command};
use predicates::str::contains;
use test_dir::TempDir;

#[test]
fn noargs() {
    assert("", &[]).stdout("").stderr("");
}

#[test]
fn noargs_echo() {
    assert("abc\ndef", &[]).stdout("abc\ndef\n").stderr("");
}

#[test]
fn no_output_when_command_fails() {
    assert("abc\ndef", &["false"]).stdout("").stderr("");
}

#[test]
fn keep_input_when_command_succeeds() {
    assert("abc\ndef\n", &["true"])
        .stdout("abc\ndef\n")
        .stderr("");
}

#[test]
fn no_output_when_command_succeeds_but_is_negated() {
    assert("abc\ndef", &["-n", "true"]).stdout("").stderr("");
}

#[test]
fn keep_input_when_command_fails_but_is_negated() {
    assert("abc\ndef\n", &["-n", "false"])
        .stdout("abc\ndef\n")
        .stderr("");
}

#[test]
fn each_line_gets_passed_to_command() {
    let tmp = TempDir::temp().expect("cannot create temporary directory");
    let log_file = tmp.path().join("log");
    let log_file_str = log_file
        .as_os_str()
        .to_str()
        .expect("unexpected non-utf8 in path");

    assert("abc\ndef\n", &["tee", "-a", log_file_str]);
    assert_eq!("abcdef", std::fs::read_to_string(log_file).unwrap());
}

#[test]
fn stdout_propagates_by_default() {
    assert("abc\ndef\n", &["echo", "x"])
        .stdout("x\nabc\nx\ndef\n")
        .stderr("");
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
        .stdout("abc\ndef\n")
        .stderr("");
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
        .stdout("x\nx\n")
        .stderr("");
}

#[test]
fn input_passed_to_command_when_braces_exist() {
    assert("abc\ndef\n", &["echo", "{}"])
        .stdout("abc\nabc\ndef\ndef\n")
        .stderr("");
}

#[test]
fn input_not_on_stdin_when_braces_exist() {
    let tmp = TempDir::temp().expect("cannot create temporary directory");
    let log_file = tmp.path().join("log");
    let log_file_str = log_file
        .as_os_str()
        .to_str()
        .expect("unexpected non-utf8 in path");

    assert(log_file_str, &["tee", "{}"]);
    assert_eq!("", std::fs::read_to_string(log_file).unwrap());
}

#[test]
fn input_run_as_command_via_braces() {
    assert(
        "command_that_should_not_exist_on_the_system\necho\n",
        &["{}"],
    )
    .stdout("\necho\n")
    .stderr(contains("Error"));
}

#[test]
fn input_run_as_command_is_split() {
    assert("echo abcd", &["--map", "{}"])
        .stdout("abcd\n")
        .stderr("");
}

#[test]
fn empty_input_run_as_command_does_not_crash() {
    assert("\n", &["{}"]).stdout("\n").stderr("");
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
    let mut cmd = Command::from_std(std::process::Command::new("/bin/sh"));
    cmd.args(&["-c", concat!(env!("CARGO_BIN_EXE_fpipe"), " | head -n 10")])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[test]
fn first_false_does_not_stop_processing() {
    assert("1\n2\n3\n", &["-q", "expr", "2", "!=", "{}"])
        .stdout("1\n3\n")
        .stderr("");
}

fn assert(input: &str, args: &[&str]) -> Assert {
    let mut cmd = Command::cargo_bin(env!("CARGO_BIN_EXE_fpipe")).unwrap();
    cmd.args(args).write_stdin(input).assert().success()
}
