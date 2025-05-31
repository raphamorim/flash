/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_script_file_execution_with_positional_args() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("test_script.sh");

    // Create a test script that uses positional parameters
    fs::write(
        &script_path,
        r#"echo "Script: $0, First: $1, Second: $2, Count: $#""#,
    )
    .unwrap();

    // Execute the script with arguments
    let output = Command::new("cargo")
        .args(["run", "--release", "--"])
        .arg(&script_path)
        .arg("hello")
        .arg("world")
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = format!(
        "Script: {}, First: hello, Second: world, Count: 2\n",
        script_path.display()
    );

    assert_eq!(stdout, expected);
    assert!(output.status.success());
}

#[test]
fn test_piped_input_with_positional_args() {
    // Test piped input with positional arguments using echo
    let output = Command::new("sh")
        .arg("-c")
        .arg("echo 'echo \"Piped: $1, $2, $3, Count: $#\"' | cargo run --release -- arg1 arg2 arg3")
        .output()
        .expect("Failed to execute test");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(stdout, "Piped: arg1, arg2, arg3, Count: 3\n");
    assert!(output.status.success());
}

#[test]
fn test_script_file_with_special_variables() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("special_vars.sh");

    // Create a script that tests all special variables
    fs::write(
        &script_path,
        r#"echo "All args: $@"
echo "All args alt: $*"
echo "Arg count: $#"
echo "Individual: $1 $2 $3""#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["run", "--release", "--"])
        .arg(&script_path)
        .arg("first")
        .arg("second")
        .arg("third")
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = "All args: first second third\nAll args alt: first second third\nArg count: 3\nIndividual: first second third\n";

    assert_eq!(stdout, expected);
    assert!(output.status.success());
}

#[test]
fn test_script_file_with_no_args() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("no_args.sh");

    // Create a script that handles no arguments
    fs::write(
        &script_path,
        r#"echo "Script: $0"
echo "First arg (should be empty): '$1'"
echo "Count: $#"
echo "All args: '$@'""#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["run", "--release", "--"])
        .arg(&script_path)
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = format!(
        "Script: {}\nFirst arg (should be empty): ''\nCount: 0\nAll args: ''\n",
        script_path.display()
    );

    assert_eq!(stdout, expected);
    assert!(output.status.success());
}

#[test]
fn test_script_file_with_spaces_in_args() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("spaces.sh");

    // Create a script that handles arguments with spaces
    fs::write(
        &script_path,
        r#"echo "First: '$1'"
echo "Second: '$2'"
echo "All: '$@'""#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["run", "--release", "--"])
        .arg(&script_path)
        .arg("hello world")
        .arg("test arg")
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = "First: 'hello world'\nSecond: 'test arg'\nAll: 'hello world test arg'\n";

    assert_eq!(stdout, expected);
    assert!(output.status.success());
}

#[test]
fn test_piped_input_vs_script_file_priority() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("priority_test.sh");

    // Create a script file
    fs::write(&script_path, r#"echo "Script file executed with: $1""#).unwrap();

    // When both script file and stdin are available, script file should take priority
    // This tests the fix for the execution flow
    let output = Command::new("cargo")
        .args(["run", "--release", "--"])
        .arg(&script_path)
        .arg("script_arg")
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = "Script file executed with: script_arg\n";

    assert_eq!(stdout, expected);
    assert!(output.status.success());
}

#[test]
fn test_command_flag_execution() {
    // Test the -c flag for direct command execution
    let output = Command::new("cargo")
        .args(["run", "--release", "--"])
        .arg("-c")
        .arg("echo \"Direct command: $1\"")
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    // With -c flag, there are no positional arguments set up
    let expected = "Direct command: \n";

    assert_eq!(stdout, expected);
    assert!(output.status.success());
}

#[test]
fn test_nonexistent_script_file() {
    // Test error handling for non-existent script files
    let output = Command::new("cargo")
        .args(["run", "--release", "--"])
        .arg("nonexistent_script.sh")
        .arg("arg1")
        .output()
        .expect("Failed to execute flash");

    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(stderr.contains("Error reading script"));
    assert!(stderr.contains("nonexistent_script.sh"));
    assert!(!output.status.success());
}

#[test]
fn test_positional_parameters_with_variable_expansion() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("var_expansion.sh");

    // Create a script that mixes positional parameters with variable expansion
    fs::write(
        &script_path,
        r#"export TEST_VAR="test_value"
echo "Var: $TEST_VAR, Arg: $1"
echo "Mixed: ${TEST_VAR}_${1}_suffix""#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["run", "--release", "--"])
        .arg(&script_path)
        .arg("hello")
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = "Var: test_value, Arg: hello\nMixed: test_value_hello_suffix\n";

    assert_eq!(stdout, expected);
    assert!(output.status.success());
}

#[test]
fn test_high_numbered_positional_parameters() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("high_numbers.sh");

    // Create a script that tests higher numbered parameters
    fs::write(
        &script_path,
        r#"echo "Args: $1 $2 $3 $4 $5 $6 $7 $8 $9 ${10} ${11}"
echo "Count: $#""#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["run", "--release", "--"])
        .arg(&script_path)
        .args(["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"])
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = "Args: a b c d e f g h i j k\nCount: 11\n";

    assert_eq!(stdout, expected);
    assert!(output.status.success());
}
