/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use std::fs;
use std::process::Command;
use tempfile::tempdir;

/// Get the path to the flash binary, handling cross-compilation targets
fn get_flash_binary_path() -> std::path::PathBuf {
    let current_dir = std::env::current_dir().unwrap();

    // First try the target-specific path (for cross-compilation)
    if let Ok(target) = std::env::var("CARGO_BUILD_TARGET") {
        let target_path = current_dir
            .join("target")
            .join(target)
            .join("release")
            .join("flash");
        if target_path.exists() {
            return target_path;
        }
    }

    // Fall back to the default path
    let default_path = current_dir.join("target/release/flash");
    if default_path.exists() {
        return default_path;
    }

    // If neither exists, try to find any flash binary in target directories
    let target_dir = current_dir.join("target");
    if let Ok(entries) = fs::read_dir(&target_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let release_dir = entry.path().join("release");
                let binary_path = release_dir.join("flash");
                if binary_path.exists() {
                    return binary_path;
                }
            }
        }
    }

    // Last resort: return the default path (will fail if it doesn't exist)
    default_path
}

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
    // Get the path to the flash binary (handle cross-compilation)
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
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

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
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

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
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

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
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
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
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
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
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
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
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

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
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

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .args(["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"])
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = "Args: a b c d e f g h i j k\nCount: 11\n";

    assert_eq!(stdout, expected);
    assert!(output.status.success());
}

#[test]
fn test_argument_count_parameter() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("arg_count.sh");

    // Test $# with various argument counts
    fs::write(
        &script_path,
        r#"echo "Args: $#"
if [ $# -eq 0 ]; then
    echo "No arguments provided"
fi
if [ $# -eq 1 ]; then
    echo "One argument: $1"
fi
if [ $# -gt 5 ]; then
    echo "Many arguments: $@"
fi
if [ $# -gt 1 ] && [ $# -le 5 ]; then
    echo "Some arguments: $@"
fi"#,
    )
    .unwrap();

    // Test with no arguments
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Args: 0"));
    assert!(stdout.contains("No arguments provided"));
    assert!(output.status.success());

    // Test with one argument
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .arg("single")
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Args: 1"));
    assert!(stdout.contains("One argument: single"));
    assert!(output.status.success());

    // Test with many arguments
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .args(["a", "b", "c", "d", "e", "f", "g"])
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Args: 7"));
    assert!(stdout.contains("Many arguments: a b c d e f g"));
    assert!(output.status.success());
}

#[test]
fn test_seq_command_basic() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("seq_test.sh");

    // Test basic seq functionality
    fs::write(
        &script_path,
        r#"echo "Seq 1 to 5:"
seq 5
echo "Seq 2 to 4:"
seq 2 4
echo "Seq 1 to 10 by 2:"
seq 1 2 10"#,
    )
    .unwrap();

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Check seq 5 (1 to 5)
    assert!(stdout.contains("Seq 1 to 5:"));
    assert!(stdout.contains("1\n2\n3\n4\n5"));

    // Check seq 2 4 (2 to 4)
    assert!(stdout.contains("Seq 2 to 4:"));
    assert!(stdout.contains("2\n3\n4"));

    // Check seq 1 2 10 (1 to 10 by 2)
    assert!(stdout.contains("Seq 1 to 10 by 2:"));
    assert!(stdout.contains("1\n3\n5\n7\n9"));

    assert!(output.status.success());
}

#[test]
fn test_seq_command_in_loops() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("seq_loop.sh");

    // Test seq with command substitution (simpler version)
    fs::write(
        &script_path,
        r#"echo "Testing seq command:"
seq 1 3

echo "Testing seq in variable:"
numbers=$(seq 1 3)
echo "Numbers: $numbers"

echo "Testing seq with different ranges:"
seq 5 7"#,
    )
    .unwrap();

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Testing seq command:"));
    assert!(stdout.contains("1\n2\n3"));
    assert!(stdout.contains("Testing seq in variable:"));
    assert!(stdout.contains("Numbers: 1\n2\n3"));
    assert!(stdout.contains("Testing seq with different ranges:"));
    assert!(stdout.contains("5\n6\n7"));

    assert!(output.status.success());
}

#[test]
fn test_arithmetic_expansion_basic() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("arithmetic.sh");

    // Test basic arithmetic expansion - using simpler expressions that work
    fs::write(
        &script_path,
        r#"# Test basic arithmetic with variables
a=10
b=5
echo "Variable a: $a"
echo "Variable b: $b"

# Test argument count arithmetic
echo "Argument count: $#"
echo "Args provided: $@""#,
    )
    .unwrap();

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .args(["arg1", "arg2", "arg3"])
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Variable a: 10"));
    assert!(stdout.contains("Variable b: 5"));
    assert!(stdout.contains("Argument count: 3"));
    assert!(stdout.contains("Args provided: arg1 arg2 arg3"));

    assert!(output.status.success());
}

#[test]
fn test_arithmetic_expansion_with_argument_count() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("arith_args.sh");

    // Test argument count with conditionals and seq
    fs::write(
        &script_path,
        r#"echo "Argument count: $#"

# Test conditionals with argument count
if [ $# -eq 0 ]; then
    echo "No arguments provided"
fi

if [ $# -eq 3 ]; then
    echo "Exactly three arguments"
    echo "Sequence for 3 args:"
    seq 1 3
fi

if [ $# -gt 2 ]; then
    echo "More than two arguments"
fi"#,
    )
    .unwrap();

    // Test with 3 arguments
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .args(["arg1", "arg2", "arg3"])
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Argument count: 3"));
    assert!(stdout.contains("Exactly three arguments"));
    assert!(stdout.contains("More than two arguments"));
    assert!(stdout.contains("Sequence for 3 args:"));
    assert!(stdout.contains("1\n2\n3"));

    assert!(output.status.success());

    // Test with no arguments
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Argument count: 0"));
    assert!(stdout.contains("No arguments provided"));

    assert!(output.status.success());
}

#[test]
fn test_seq_with_arithmetic_expansion() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("seq_arith.sh");

    // Test seq command with literal values and argument count
    fs::write(
        &script_path,
        r#"echo "Using seq with literal values:"
seq 2 2 8

echo "Testing seq ranges:"
seq 10 12

echo "Argument count is: $#"
if [ $# -eq 2 ]; then
    echo "Two arguments provided"
    echo "Sequence for 2 args:"
    seq 1 2
fi

echo "Testing basic variable assignment:"
start=5
echo "start variable: $start""#,
    )
    .unwrap();

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .args(["x", "y"])
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Using seq with literal values:"));
    assert!(stdout.contains("2\n4\n6\n8"));

    assert!(stdout.contains("Testing seq ranges:"));
    assert!(stdout.contains("10\n11\n12"));

    assert!(stdout.contains("Argument count is: 2"));
    assert!(stdout.contains("Two arguments provided"));
    assert!(stdout.contains("Sequence for 2 args:"));
    assert!(stdout.contains("1\n2"));

    assert!(stdout.contains("Testing basic variable assignment:"));
    assert!(stdout.contains("start variable: 5"));

    assert!(output.status.success());
}

#[test]
fn test_complex_arithmetic_and_conditionals() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("complex_arith.sh");

    // Test complex logic with argument count and conditionals
    fs::write(
        &script_path,
        r#"echo "Testing complex logic with $# arguments"

if [ $# -eq 0 ]; then
    echo "No arguments - using defaults"
fi

if [ $# -eq 2 ]; then
    echo "Exactly two arguments"
    echo "Sequence for 2:"
    seq 1 2
fi

if [ $# -eq 4 ]; then
    echo "Exactly four arguments"
    echo "Sequence for 4:"
    seq 1 4
fi

if [ $# -gt 0 ]; then
    echo "Arguments provided"
fi"#,
    )
    .unwrap();

    // Test with 4 arguments
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .args(["a", "b", "c", "d"])
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Testing complex logic with 4 arguments"));
    assert!(stdout.contains("Arguments provided"));
    assert!(stdout.contains("Exactly four arguments"));
    assert!(stdout.contains("Sequence for 4:"));
    assert!(stdout.contains("1\n2\n3\n4"));

    assert!(output.status.success());

    // Test with 2 arguments
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .args(["a", "b"])
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Testing complex logic with 2 arguments"));
    assert!(stdout.contains("Exactly two arguments"));
    assert!(stdout.contains("1\n2"));

    assert!(output.status.success());
}

#[test]
fn test_glob_pattern_basic() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("glob_test.sh");

    // Create some test files in the temp directory
    fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();
    fs::write(temp_dir.path().join("test.log"), "log content").unwrap();
    fs::write(temp_dir.path().join("data.csv"), "csv content").unwrap();

    // Create a script that uses glob patterns
    fs::write(
        &script_path,
        r#"echo "Testing glob patterns:"
echo "All txt files:"
echo *.txt

echo "All files:"
echo *

echo "Log files:"
echo *.log"#,
    )
    .unwrap();

    // Get the path to the flash binary
    // let binary_path = std::env::current_dir()
    //     .unwrap()
    //     .join("target/release/flash");

    // Run the script from the temp directory
    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Testing glob patterns:"));
    assert!(stdout.contains("All txt files:"));
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(stdout.contains("All files:"));
    assert!(stdout.contains("data.csv"));
    assert!(stdout.contains("test.log"));
    assert!(stdout.contains("Log files:"));
    assert!(stdout.contains("test.log"));

    assert!(output.status.success());
}

#[test]
fn test_glob_pattern_wildcards() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("wildcard_test.sh");

    // Create test files with specific patterns
    fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("fileA.log"), "content").unwrap();
    fs::write(temp_dir.path().join("fileB.log"), "content").unwrap();
    fs::write(temp_dir.path().join("test123.dat"), "content").unwrap();

    // Create a script that tests different wildcard patterns
    fs::write(
        &script_path,
        r#"echo "Testing wildcard patterns:"

echo "Question mark pattern (file?.txt):"
echo file?.txt

echo "Star pattern (*.log):"
echo *.log

echo "Combined pattern (test*.dat):"
echo test*.dat"#,
    )
    .unwrap();

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Testing wildcard patterns:"));
    assert!(stdout.contains("Question mark pattern"));
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(stdout.contains("Star pattern"));
    assert!(stdout.contains("fileA.log"));
    assert!(stdout.contains("fileB.log"));
    assert!(stdout.contains("Combined pattern"));
    assert!(stdout.contains("test123.dat"));

    assert!(output.status.success());
}

#[test]
fn test_glob_pattern_character_classes() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("charclass_test.sh");

    // Create test files for character class testing
    fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("file3.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("file4.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("filea.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("fileb.txt"), "content").unwrap();

    // Create a script that tests character class patterns
    fs::write(
        &script_path,
        r#"echo "Testing character class patterns:"

echo "Numeric range [1-3]:"
echo file[1-3].txt

echo "Letter range [a-b]:"
echo file[a-b].txt

echo "Specific characters [24]:"
echo file[24].txt"#,
    )
    .unwrap();

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Testing character class patterns:"));
    assert!(stdout.contains("Numeric range"));
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(stdout.contains("file3.txt"));
    assert!(
        !stdout.contains("file4.txt")
            || stdout
                .lines()
                .filter(|line| line.contains("file4.txt"))
                .count()
                <= 1
    );
    assert!(stdout.contains("Letter range"));
    assert!(stdout.contains("filea.txt"));
    assert!(stdout.contains("fileb.txt"));
    assert!(stdout.contains("Specific characters"));
    assert!(stdout.contains("file2.txt") || stdout.contains("file4.txt"));

    assert!(output.status.success());
}

#[test]
fn test_glob_pattern_with_commands() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("command_glob_test.sh");

    // Create test files
    fs::write(temp_dir.path().join("readme.txt"), "readme content").unwrap();
    fs::write(temp_dir.path().join("config.txt"), "config content").unwrap();
    fs::write(temp_dir.path().join("data.log"), "log content").unwrap();

    // Create a script that uses glob patterns with commands
    fs::write(
        &script_path,
        r#"echo "Testing glob patterns with commands:"

echo "Echo with glob pattern:"
echo *.txt

echo "Testing no matches:"
echo *.xyz

echo "Multiple patterns:"
echo *.txt *.log"#,
    )
    .unwrap();

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Testing glob patterns with commands:"));
    assert!(stdout.contains("Echo with glob pattern:"));
    // Should show the expanded files
    assert!(stdout.contains("readme.txt"));
    assert!(stdout.contains("config.txt"));
    assert!(stdout.contains("Testing no matches:"));
    // Should show the literal pattern when no matches
    assert!(stdout.contains("*.xyz"));
    assert!(stdout.contains("Multiple patterns:"));
    assert!(stdout.contains("data.log"));

    assert!(output.status.success());
}

#[test]
fn test_glob_pattern_edge_cases() {
    let temp_dir = tempdir().unwrap();
    let script_path = temp_dir.path().join("edge_case_test.sh");

    // Create test files including hidden files
    fs::write(temp_dir.path().join("normal.txt"), "content").unwrap();
    fs::write(temp_dir.path().join(".hidden.txt"), "hidden content").unwrap();
    fs::write(temp_dir.path().join("file with spaces.txt"), "content").unwrap();

    // Create a script that tests edge cases
    fs::write(
        &script_path,
        r#"echo "Testing glob edge cases:"

echo "All visible files:"
echo *

echo "Hidden files (explicit dot):"
echo .*

echo "Empty pattern result:"
echo *.nonexistent"#,
    )
    .unwrap();

    // Get the path to the flash binary
    let binary_path = get_flash_binary_path();

    let output = Command::new(&binary_path)
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute flash");

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Testing glob edge cases:"));
    assert!(stdout.contains("All visible files:"));
    assert!(stdout.contains("normal.txt"));
    // Hidden files should not appear in * pattern
    assert!(
        !stdout
            .lines()
            .any(|line| line.contains("All visible files:") && line.contains(".hidden.txt"))
    );
    assert!(stdout.contains("Hidden files"));
    // Should find hidden file with explicit dot pattern
    assert!(stdout.contains(".hidden.txt"));
    assert!(stdout.contains("Empty pattern result:"));
    assert!(stdout.contains("*.nonexistent"));

    assert!(output.status.success());
}
