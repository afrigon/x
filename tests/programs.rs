use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use libtest_mimic::{Arguments, Completion, Failed, Trial};
use similar::TextDiff;

const COMPILER: &str = env!("CARGO_BIN_EXE_x");
const REQUIRED_TOOLS: [&str; 2] = ["llc", "clang"];
const RECORD_VARIABLE: &str = "X_RECORD_SNAPSHOTS";

fn main() {
    let arguments = Arguments::from_args();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs");
    let missing_tool = REQUIRED_TOOLS.into_iter().find(|tool| !is_available(tool));
    let trials = programs(&root)
        .into_iter()
        .map(|program| {
            let name = program
                .strip_prefix(&root)
                .expect("program lives under the root")
                .with_extension("")
                .to_string_lossy()
                .into_owned();
            Trial::ignorable_test(name, move || match missing_tool {
                Some(tool) => Ok(Completion::ignored_with(format!(
                    "{tool} not found in PATH"
                ))),
                None => check(&program).map(|()| Completion::Completed),
            })
        })
        .collect();
    libtest_mimic::run(&arguments, trials).exit();
}

fn programs(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(directory).expect("cannot read the programs directory") {
        let path = entry.expect("cannot read a directory entry").path();
        if path.is_dir() {
            found.extend(programs(&path));
        } else if path.extension().is_some_and(|extension| extension == "x") {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn is_available(tool: &str) -> bool {
    !matches!(
        Command::new(tool).arg("--version").output(),
        Err(error) if error.kind() == io::ErrorKind::NotFound
    )
}

struct Expectation {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

fn expectation(program: &Path) -> Result<Expectation, Failed> {
    let source = fs::read_to_string(program)?;
    let mut exit_code = 0;
    for line in source.lines() {
        let Some(directive) = line.strip_prefix("//@") else {
            break;
        };
        let (key, value) = directive
            .split_once(':')
            .ok_or_else(|| format!("malformed directive: {line}"))?;
        match key.trim() {
            "exit-code" => {
                exit_code = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("exit-code is not an integer: {line}"))?
            }
            other => return Err(format!("unknown directive: {other}").into()),
        }
    }
    Ok(Expectation {
        stdout: snapshot(&program.with_extension("stdout"))?,
        stderr: snapshot(&program.with_extension("stderr"))?,
        exit_code,
    })
}

fn snapshot(path: &Path) -> Result<String, Failed> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

fn check(program: &Path) -> Result<(), Failed> {
    let expected = expectation(program)?;
    let directory = tempfile::tempdir()?;
    let executable = directory.path().join("program");

    let build = Command::new(COMPILER)
        .arg("build")
        .arg(program)
        .arg("-o")
        .arg(&executable)
        .output()?;
    if !build.status.success() {
        return Err(format!(
            "compilation failed:\n{}",
            String::from_utf8_lossy(&build.stderr)
        )
        .into());
    }

    let output = Command::new(&executable).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit_code = output
        .status
        .code()
        .ok_or("program terminated by a signal")?;

    if std::env::var_os(RECORD_VARIABLE).is_some() {
        record(&program.with_extension("stdout"), &stdout)?;
        record(&program.with_extension("stderr"), &stderr)?;
        return Ok(());
    }

    let mut failures = Vec::new();
    if let Some(diff) = difference("stdout", &expected.stdout, &stdout) {
        failures.push(diff);
    }
    if let Some(diff) = difference("stderr", &expected.stderr, &stderr) {
        failures.push(diff);
    }
    if exit_code != expected.exit_code {
        failures.push(format!(
            "exit code: expected {}, actual {exit_code}",
            expected.exit_code
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n").into())
    }
}

fn difference(stream: &str, expected: &str, actual: &str) -> Option<String> {
    if expected == actual {
        return None;
    }
    let diff = TextDiff::from_lines(expected, actual);
    Some(format!(
        "{stream} differs:\n{}",
        diff.unified_diff().header("expected", "actual")
    ))
}

fn record(path: &Path, contents: &str) -> Result<(), Failed> {
    if contents.is_empty() {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    } else {
        Ok(fs::write(path, contents)?)
    }
}
