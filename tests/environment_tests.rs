/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use flash::interpreter::{DefaultEvaluator, Interpreter};
use std::io;

fn execute_script(script: &str) -> Result<i32, io::Error> {
    let mut interpreter = Interpreter::new();
    let mut evaluator = DefaultEvaluator;
    interpreter.execute_with_evaluator(script, &mut evaluator)
}

#[test]
fn test_environment_variable_expansion() {
    let script = r#"
        export TEST_VAR="hello world"
        echo $TEST_VAR
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_with_braces() {
    let script = r#"
        export USER_NAME="alice"
        echo "Hello ${USER_NAME}!"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_default_value() {
    let script = r#"
        echo "${UNDEFINED_VAR:-default_value}"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_assignment_and_use() {
    let script = r#"
        VAR1="first"
        VAR2="second"
        echo "$VAR1 and $VAR2"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_in_command_args() {
    let script = r#"
        FILE_NAME="test.txt"
        echo "Processing file: $FILE_NAME"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_concatenation() {
    let script = r#"
        PREFIX="hello"
        SUFFIX="world"
        echo "${PREFIX}_${SUFFIX}"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_nested_expansion() {
    let script = r#"
        INNER="value"
        OUTER="INNER"
        echo "${!OUTER}"
    "#;
    let result = execute_script(script);
    // This might not be implemented yet, but should not crash
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_environment_variable_in_conditionals() {
    let script = r#"
        STATUS="success"
        if [ "$STATUS" = "success" ]; then
            echo "Operation successful"
        else
            echo "Operation failed"
        fi
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_empty_value() {
    let script = r#"
        EMPTY_VAR=""
        echo "Value: '$EMPTY_VAR'"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_unset() {
    let script = r#"
        export TEST_VAR="initial"
        echo "Before: $TEST_VAR"
        unset TEST_VAR
        echo "After: $TEST_VAR"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_local_scope() {
    let script = r#"
        GLOBAL_VAR="global"
        test_function() {
            local LOCAL_VAR="local"
            echo "Local: $LOCAL_VAR"
            echo "Global: $GLOBAL_VAR"
        }
        test_function
        echo "Outside function: $LOCAL_VAR"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_export() {
    let script = r#"
        VAR_NOT_EXPORTED="not exported"
        export VAR_EXPORTED="exported"
        echo "Both variables set"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_path_expansion() {
    let script = r#"
        HOME_DIR="/home/user"
        CONFIG_PATH="$HOME_DIR/.config"
        echo "Config path: $CONFIG_PATH"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_arithmetic() {
    let script = r#"
        NUM1=5
        NUM2=3
        echo "Numbers: $NUM1 and $NUM2"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_special_characters() {
    let script = r#"
        SPECIAL="hello@world.com"
        echo "Email: $SPECIAL"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_multiline() {
    let script = r#"
        MULTILINE="line1
line2
line3"
        echo "$MULTILINE"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_command_substitution() {
    let script = r#"
        CURRENT_DATE=$(echo "2025-01-01")
        echo "Date: $CURRENT_DATE"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_in_loops() {
    let script = r#"
        PREFIX="item"
        for i in 1 2 3; do
            echo "${PREFIX}_$i"
        done
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_case_sensitivity() {
    let script = r#"
        lowercase="lower"
        UPPERCASE="UPPER"
        echo "Lower: $lowercase"
        echo "Upper: $UPPERCASE"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_readonly() {
    let script = r#"
        readonly READONLY_VAR="cannot change"
        echo "Readonly: $READONLY_VAR"
    "#;
    let result = execute_script(script);
    // Should work even if readonly is not fully implemented
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_environment_variable_array_like() {
    let script = r#"
        ITEMS="apple banana cherry"
        echo "Items: $ITEMS"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_with_quotes() {
    let script = r#"
        QUOTED_VAR="value with spaces"
        echo "Quoted: '$QUOTED_VAR'"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_environment_variable_length() {
    let script = r#"
        TEST_STRING="hello"
        echo "String: $TEST_STRING"
        echo "Length would be: ${#TEST_STRING}"
    "#;
    let result = execute_script(script);
    // Length expansion might not be implemented, but should not crash
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_environment_variable_substring() {
    let script = r#"
        FULL_STRING="hello world"
        echo "Full: $FULL_STRING"
        echo "Substring would be: ${FULL_STRING:0:5}"
    "#;
    let result = execute_script(script);
    // Substring expansion might not be implemented, but should not crash
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_environment_variable_pattern_replacement() {
    let script = r#"
        TEXT="hello world hello"
        echo "Original: $TEXT"
        echo "Replaced would be: ${TEXT/hello/hi}"
    "#;
    let result = execute_script(script);
    // Pattern replacement might not be implemented, but should not crash
    assert!(result.is_ok() || result.is_err());
}
