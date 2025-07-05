/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

#[cfg(feature = "formatter")]
use flash::formatter::Formatter;
use flash::lexer::Lexer;
use flash::parser::Parser;

#[cfg(feature = "formatter")]
fn format_script(script: &str) -> String {
    let lexer = Lexer::new(script);
    let mut parser = Parser::new(lexer);
    let ast = parser.parse_script();
    let mut formatter = Formatter::new();
    formatter.format(&ast)
}

#[cfg(feature = "formatter")]
#[test]
fn test_formatter_simple_command() {
    let input = "echo hello";
    let output = format_script(input);
    assert_eq!(output, "echo hello\n");
}

#[cfg(feature = "formatter")]
#[test]
fn test_formatter_pipeline() {
    let input = "ls|grep test";
    let output = format_script(input);
    assert_eq!(output, "ls | grep test\n");
}

#[cfg(feature = "formatter")]
#[test]
fn test_formatter_if_statement() {
    let input = "if [ -f file ];then echo found;fi";
    let output = format_script(input);
    assert!(output.contains("if"));
    assert!(output.contains("then"));
    assert!(output.contains("fi"));
}

#[cfg(feature = "formatter")]
#[test]
fn test_formatter_for_loop() {
    let input = "for i in 1 2 3;do echo $i;done";
    let output = format_script(input);
    assert!(output.contains("for"));
    assert!(output.contains("do"));
    assert!(output.contains("done"));
}

#[cfg(feature = "formatter")]
#[test]
fn test_formatter_function() {
    let input = "function test(){echo hello;}";
    let output = format_script(input);
    assert!(output.contains("function"));
    assert!(output.contains("test"));
}
