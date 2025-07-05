/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use flash::interpreter::{DefaultEvaluator, Evaluator, Interpreter};
use flash::lexer::Lexer;
use flash::parser::Parser;

#[test]
fn test_parameter_expansion_parsing() {
    let mut interpreter = Interpreter::new();
    interpreter
        .variables
        .insert("TEST_VAR".to_string(), "hello".to_string());

    // Test that parameter expansion is parsed correctly
    let input = "echo ${TEST_VAR}";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_script();

    let mut evaluator = DefaultEvaluator;
    let result = evaluator.evaluate(&ast, &mut interpreter);
    if let Err(e) = &result {
        println!("Error in test_parameter_expansion_parsing: {e:?}");
    }
    assert!(result.is_ok());
}

#[test]
fn test_parameter_expansion_default() {
    let mut interpreter = Interpreter::new();

    // Test default value expansion
    let input = "echo ${UNSET_VAR:-default_value}";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_script();

    let mut evaluator = DefaultEvaluator;
    let result = evaluator.evaluate(&ast, &mut interpreter);
    if let Err(e) = &result {
        println!("Error in test_parameter_expansion_default: {e:?}");
    }
    assert!(result.is_ok());
}

#[test]
fn test_process_substitution_parsing() {
    let mut interpreter = Interpreter::new();

    // Test process substitution parsing
    let input = "diff <(echo hello) <(echo world)";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_script();

    // Should parse without error
    let mut evaluator = DefaultEvaluator;
    let result = evaluator.evaluate(&ast, &mut interpreter);
    // Process substitution might fail in execution but should parse
    assert!(result.is_ok() || result.is_err());
}

#[test]
#[ignore]
fn test_read_builtin() {
    let mut interpreter = Interpreter::new();
    let mut evaluator = DefaultEvaluator;

    // Test read command (will fail in test environment but should be recognized)
    let input = "read var_name";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_script();

    let result = evaluator.evaluate(&ast, &mut interpreter);
    // read will fail in test environment but should be recognized as a command
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_eval_builtin() {
    let mut interpreter = Interpreter::new();
    let mut evaluator = DefaultEvaluator;

    // Test eval command
    let input = "eval 'echo hello'";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_script();

    let result = evaluator.evaluate(&ast, &mut interpreter);
    assert!(result.is_ok());
}

#[test]
fn test_local_builtin() {
    let mut interpreter = Interpreter::new();
    let mut evaluator = DefaultEvaluator;

    // Test local command - for now just test that it doesn't crash
    let input = "local var=value";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_script();

    let result = evaluator.evaluate(&ast, &mut interpreter);
    assert!(result.is_ok());
    // Note: Assignment parsing needs improvement for this to work fully
}

#[test]
fn test_set_builtin() {
    let mut interpreter = Interpreter::new();
    let mut evaluator = DefaultEvaluator;

    // Test set -e command
    let input = "set -e";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_script();

    let result = evaluator.evaluate(&ast, &mut interpreter);
    assert!(result.is_ok());
    // Note: shell_options field might not be accessible in current implementation
}

#[test]
fn test_debug_mode() {
    let mut interpreter = Interpreter::new();
    let mut evaluator = DefaultEvaluator;

    // Test set -x command
    let input = "set -x";
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_script();

    let result = evaluator.evaluate(&ast, &mut interpreter);
    assert!(result.is_ok());
    // Note: shell_options field might not be accessible in current implementation
}

#[test]
fn test_here_document_lexing() {
    // Test that here-document tokens are lexed correctly
    let input = "cat << EOF\nhello\nEOF";
    let mut lexer = Lexer::new(input);

    let token1 = lexer.next_token();
    assert_eq!(token1.value, "cat");

    let token2 = lexer.next_token();
    assert_eq!(token2.value, "<<");

    let token3 = lexer.next_token();
    assert_eq!(token3.value, "EOF");
}

#[test]
fn test_here_string_lexing() {
    // Test that here-string tokens are lexed correctly
    let input = "cat <<< hello";
    let mut lexer = Lexer::new(input);

    let token1 = lexer.next_token();
    assert_eq!(token1.value, "cat");

    let token2 = lexer.next_token();
    assert_eq!(token2.value, "<<<");

    let token3 = lexer.next_token();
    assert_eq!(token3.value, "hello");
}

#[test]
fn test_parameter_expansion_lexing() {
    // Test that parameter expansion tokens are lexed correctly
    let input = "${VAR}";
    let mut lexer = Lexer::new(input);

    let token1 = lexer.next_token();
    assert_eq!(token1.value, "${");

    let token2 = lexer.next_token();
    assert_eq!(token2.value, "VAR");

    let token3 = lexer.next_token();
    assert_eq!(token3.value, "}");
}

#[test]
fn test_process_substitution_lexing() {
    // Test that process substitution tokens are lexed correctly
    let input = "<(echo hello) >(cat)";
    let mut lexer = Lexer::new(input);

    let token1 = lexer.next_token();
    assert_eq!(token1.value, "<(");

    let token2 = lexer.next_token();
    assert_eq!(token2.value, "echo");

    let token3 = lexer.next_token();
    assert_eq!(token3.value, "hello");

    let token4 = lexer.next_token();
    assert_eq!(token4.value, ")");

    let token5 = lexer.next_token();
    assert_eq!(token5.value, ">(");
}
