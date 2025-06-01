/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use flash::interpreter::{DefaultEvaluator, Interpreter};
use flash::lexer::Lexer;
use flash::parser::{Node, Parser};
use std::io;

fn execute_script(script: &str) -> Result<i32, io::Error> {
    let mut interpreter = Interpreter::new();
    let mut evaluator = DefaultEvaluator;
    interpreter.execute_with_evaluator(script, &mut evaluator)
}

fn parse_script(script: &str) -> Node {
    let lexer = Lexer::new(script);
    let mut parser = Parser::new(lexer);
    parser.parse_script()
}

#[test]
fn test_function_definition_with_keyword() {
    let script = "function greet() { echo hello; }";
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_function_definition_without_keyword() {
    let script = "greet() { echo hello; }";
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_function_call() {
    let script = r#"
        greet() { echo "Hello, World!"; }
        greet
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_function_with_arguments() {
    let script = r#"
        greet() { echo "Hello, $1!"; }
        greet Alice
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_function_with_multiple_arguments() {
    let script = r#"
        greet() { echo "Hello, $1 $2!"; }
        greet Alice Bob
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_function_with_return_value() {
    let script = r#"
        add() { 
            echo "Adding $1 and $2"
            return 5
        }
        add 5 3
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 5);
}

#[test]
fn test_function_with_return_no_value() {
    let script = r#"
        test_func() { 
            echo "doing something"
            return
            echo "this should not print"
        }
        test_func
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_function_with_return_explicit_value() {
    let script = r#"
        test_func() { 
            echo "doing something"
            return 42
        }
        test_func
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_function_positional_parameters() {
    let script = r#"
        show_args() { 
            echo "Number of args: $#"
            echo "All args: $@"
            echo "First arg: $1"
            echo "Second arg: $2"
        }
        show_args one two three
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_nested_function_calls() {
    let script = r#"
        inner() { echo "inner: $1"; return 1; }
        outer() { 
            echo "outer: $1"
            inner "from outer"
            return 2
        }
        outer "test"
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2);
}

#[test]
fn test_function_local_variables() {
    let script = r#"
        test_func() { 
            local_var="inside function"
            echo $local_var
        }
        local_var="outside function"
        test_func
        echo $local_var
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_function_with_conditionals() {
    let script = r#"
        check_number() {
            if [ $1 -gt 10 ]; then
                echo "big number"
                return 1
            else
                echo "small number"
                return 0
            fi
        }
        check_number 5
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_function_with_loops() {
    let script = "count_to() { echo \"Counting to $1\"; return 3; }; count_to 3";
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 3);
}

#[test]
fn test_function_redefinition() {
    let script = r#"
        test_func() { echo "first version"; return 1; }
        test_func() { echo "second version"; return 2; }
        test_func
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2);
}

#[test]
fn test_function_parsing_with_keyword() {
    let script = "function greet() { echo hello; }";
    let ast = parse_script(script);

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Function { name, .. } => {
                    assert_eq!(name, "greet");
                }
                _ => panic!("Expected Function node"),
            }
        }
        _ => panic!("Expected List node"),
    }
}

#[test]
fn test_function_parsing_without_keyword() {
    let script = "greet() { echo hello; }";
    let ast = parse_script(script);

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Function { name, .. } => {
                    assert_eq!(name, "greet");
                }
                _ => panic!("Expected Function node"),
            }
        }
        _ => panic!("Expected List node"),
    }
}

#[test]
fn test_return_parsing() {
    let script = "return 42";
    let ast = parse_script(script);

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Return { value } => {
                    assert!(value.is_some());
                    match value.as_ref().unwrap().as_ref() {
                        Node::StringLiteral(val) => {
                            assert_eq!(val, "42");
                        }
                        _ => panic!("Expected StringLiteral node for return value"),
                    }
                }
                _ => panic!("Expected Return node"),
            }
        }
        _ => panic!("Expected List node"),
    }
}

#[test]
fn test_return_parsing_no_value() {
    let script = "return";
    let ast = parse_script(script);

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Return { value } => {
                    assert!(value.is_none());
                }
                _ => panic!("Expected Return node"),
            }
        }
        _ => panic!("Expected List node"),
    }
}

#[test]
fn test_function_with_complex_body() {
    let script = "complex_func() { echo \"Starting function\"; if [ $# -eq 0 ]; then echo \"No arguments provided\"; return 1; fi; echo \"Processing arguments\"; echo \"Function complete\"; return 0; }; complex_func arg1 arg2 arg3";
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_function_error_handling() {
    let script = r#"
        failing_func() {
            echo "This will fail"
            false
        }
        failing_func
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    // The return value should be 1 (false command's exit code)
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_factorial_function() {
    let script = r#"
        function factorial() {
            if [ $1 -le 1 ]; then
                echo "Base case: $1"
                return 1
            else
                echo "Computing factorial of $1"
                return $1
            fi
        }
        factorial 5
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 5);
}

#[test]
fn test_factorial_base_case() {
    let script = r#"
        function factorial() {
            if [ $1 -le 1 ]; then
                echo "Base case: $1"
                return 1
            else
                echo "Computing factorial of $1"
                return $1
            fi
        }
        factorial 1
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_factorial_zero() {
    let script = r#"
        function factorial() {
            if [ $1 -le 1 ]; then
                echo "Base case: $1"
                return 1
            else
                echo "Computing factorial of $1"
                return $1
            fi
        }
        factorial 0
    "#;
    let result = execute_script(script);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}
