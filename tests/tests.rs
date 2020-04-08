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
fn stdout_not_propagated_silent_short() {
    stdout_not_propagated("-s");
}

#[test]
fn stdout_not_propagated_silent_long() {
    stdout_not_propagated("--silent");
}

fn stdout_not_propagated(silent: &str) {
    assert("abc\ndef\n", &[silent, "echo", "x"])
        .stdout()
        .is("abc\ndef\n")
        .stderr()
        .is("")
        .unwrap();
}

fn assert(input: &str, args: &[&str]) -> Assert {
    assert_cli::Assert::command(&[env!("CARGO_BIN_EXE_fpipe")])
        .with_args(args)
        .stdin(input)
}
