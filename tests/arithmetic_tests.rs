/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use flash::interpreter::Interpreter;
use flash::lexer::Lexer;
use flash::parser::Parser;

#[test]
fn test_arithmetic_expansion_basic() {
    let mut interpreter = Interpreter::new();
    
    // Test basic arithmetic expansion
    let result = interpreter.execute("echo $((2 + 3))");
    assert!(result.is_ok());
    
    // Test with variables
    interpreter.variables.insert("x".to_string(), "5".to_string());
    interpreter.variables.insert("y".to_string(), "3".to_string());
    let result = interpreter.execute("echo $(($x + $y))");
    assert!(result.is_ok());
}

#[test]
fn test_arithmetic_expansion_operations() {
    let mut interpreter = Interpreter::new();
    
    // Test various arithmetic operations
    let test_cases = vec![
        ("echo $((5 + 3))", "8"),
        ("echo $((10 - 4))", "6"),
        ("echo $((6 * 7))", "42"),
        ("echo $((15 / 3))", "5"),
        ("echo $((17 % 5))", "2"),
    ];
    
    for (command, _expected) in test_cases {
        let result = interpreter.execute(command);
        assert!(result.is_ok(), "Failed to execute: {}", command);
    }
}

#[test]
fn test_arithmetic_expansion_with_variables() {
    let mut interpreter = Interpreter::new();
    
    // Set up variables
    interpreter.variables.insert("a".to_string(), "10".to_string());
    interpreter.variables.insert("b".to_string(), "5".to_string());
    
    let test_cases = vec![
        "echo $(($a + $b))",
        "echo $(($a - $b))",
        "echo $(($a * $b))",
        "echo $(($a / $b))",
        "echo $(($a % $b))",
    ];
    
    for command in test_cases {
        let result = interpreter.execute(command);
        assert!(result.is_ok(), "Failed to execute: {}", command);
    }
}

#[test]
fn test_arithmetic_expansion_parentheses() {
    let mut interpreter = Interpreter::new();
    
    let test_cases = vec![
        "echo $((2 * (3 + 4)))",
        "echo $(((10 + 5) / 3))",
        "echo $((2 + 3 * 4))",
        "echo $(((2 + 3) * 4))",
    ];
    
    for command in test_cases {
        let result = interpreter.execute(command);
        assert!(result.is_ok(), "Failed to execute: {}", command);
    }
}

#[test]
fn test_arithmetic_command_basic() {
    let mut interpreter = Interpreter::new();
    
    // Test basic arithmetic commands
    let result = interpreter.execute("(( 5 > 3 ))");
    assert_eq!(result.unwrap(), 0); // Should succeed (exit code 0)
    
    let result = interpreter.execute("(( 3 > 5 ))");
    assert_eq!(result.unwrap(), 1); // Should fail (exit code 1)
}

#[test]
fn test_arithmetic_command_operations() {
    let mut interpreter = Interpreter::new();
    
    // Test various comparison operations
    let test_cases = vec![
        ("(( 5 > 3 ))", 0),   // true
        ("(( 3 > 5 ))", 1),   // false
        ("(( 5 >= 5 ))", 0),  // true
        ("(( 3 >= 5 ))", 1),  // false
        ("(( 3 < 5 ))", 0),   // true
        ("(( 5 < 3 ))", 1),   // false
        ("(( 5 <= 5 ))", 0),  // true
        ("(( 5 <= 3 ))", 1),  // false
        ("(( 5 == 5 ))", 0),  // true
        ("(( 5 == 3 ))", 1),  // false
        ("(( 5 != 3 ))", 0),  // true
        ("(( 5 != 5 ))", 1),  // false
    ];
    
    for (command, expected_exit_code) in test_cases {
        let result = interpreter.execute(command);
        assert_eq!(result.unwrap(), expected_exit_code, "Failed for command: {}", command);
    }
}

#[test]
fn test_arithmetic_command_with_variables() {
    let mut interpreter = Interpreter::new();
    
    // Set up variables
    interpreter.variables.insert("x".to_string(), "10".to_string());
    interpreter.variables.insert("y".to_string(), "5".to_string());
    
    let test_cases = vec![
        ("(( $x > $y ))", 0),   // true
        ("(( $y > $x ))", 1),   // false
        ("(( $x == 10 ))", 0),  // true
        ("(( $y == 10 ))", 1),  // false
    ];
    
    for (command, expected_exit_code) in test_cases {
        let result = interpreter.execute(command);
        assert_eq!(result.unwrap(), expected_exit_code, "Failed for command: {}", command);
    }
}

#[test]
fn test_arithmetic_command_logical_operators() {
    let mut interpreter = Interpreter::new();
    
    let test_cases = vec![
        ("(( 1 && 1 ))", 0),   // true
        ("(( 1 && 0 ))", 1),   // false
        ("(( 0 && 1 ))", 1),   // false
        ("(( 0 && 0 ))", 1),   // false
        ("(( 1 || 1 ))", 0),   // true
        ("(( 1 || 0 ))", 0),   // true
        ("(( 0 || 1 ))", 0),   // true
        ("(( 0 || 0 ))", 1),   // false
    ];
    
    for (command, expected_exit_code) in test_cases {
        let result = interpreter.execute(command);
        assert_eq!(result.unwrap(), expected_exit_code, "Failed for command: {}", command);
    }
}

#[test]
fn test_arithmetic_command_in_conditionals() {
    let mut interpreter = Interpreter::new();
    
    // Test arithmetic commands in if statements
    let script = r#"
        x=10
        if (( x > 5 )); then
            echo "x is greater than 5"
        else
            echo "x is not greater than 5"
        fi
    "#;
    
    let result = interpreter.execute(script);
    assert!(result.is_ok());
}

#[test]
fn test_arithmetic_assignment() {
    let mut interpreter = Interpreter::new();
    
    // Test arithmetic assignment
    let result = interpreter.execute("(( x = 5 + 3 ))");
    assert!(result.is_ok());
    
    // Check that the variable was set
    assert_eq!(interpreter.variables.get("x"), Some(&"8".to_string()));
}

#[test]
fn test_arithmetic_increment_decrement() {
    let mut interpreter = Interpreter::new();
    
    // Set initial value
    interpreter.variables.insert("i".to_string(), "5".to_string());
    
    // Test increment
    let result = interpreter.execute("(( i++ ))");
    assert!(result.is_ok());
    
    // Test decrement
    let result = interpreter.execute("(( i-- ))");
    assert!(result.is_ok());
}

#[test]
fn test_arithmetic_complex_expressions() {
    let mut interpreter = Interpreter::new();
    
    interpreter.variables.insert("a".to_string(), "10".to_string());
    interpreter.variables.insert("b".to_string(), "5".to_string());
    interpreter.variables.insert("c".to_string(), "2".to_string());
    
    let test_cases = vec![
        "echo $(($a + $b * $c))",
        "echo $(($a * ($b + $c)))",
        "(( $a > $b && $b > $c ))",
        "(( $a == 10 || $b == 10 ))",
    ];
    
    for command in test_cases {
        let result = interpreter.execute(command);
        assert!(result.is_ok(), "Failed to execute: {}", command);
    }
}

#[test]
fn test_arithmetic_lexer_tokens() {
    let mut lexer = Lexer::new("(( 5 + 3 ))");
    
    let tokens: Vec<_> = std::iter::from_fn(|| {
        let token = lexer.next_token();
        if token.kind == flash::lexer::TokenKind::EOF {
            None
        } else {
            Some(token)
        }
    }).collect();
    
    // Should start with ArithCommand token
    assert_eq!(tokens[0].kind, flash::lexer::TokenKind::ArithCommand);
    assert_eq!(tokens[0].value, "((");
}

#[test]
fn test_arithmetic_parser() {
    let lexer = Lexer::new("(( x + y ))");
    let mut parser = Parser::new(lexer);
    
    let ast = parser.parse_statement();
    assert!(ast.is_some());
    
    if let Some(node) = ast {
        match node {
            flash::parser::Node::ArithmeticCommand { expression } => {
                assert_eq!(expression, "x + y");
            }
            _ => panic!("Expected ArithmeticCommand node"),
        }
    }
}

#[test]
fn test_arithmetic_expansion_parser() {
    let lexer = Lexer::new("echo $((x + y))");
    let mut parser = Parser::new(lexer);
    
    let ast = parser.parse_statement();
    assert!(ast.is_some());
    
    // The arithmetic expansion should be parsed as part of the command arguments
    if let Some(node) = ast {
        match node {
            flash::parser::Node::Command { name, args, .. } => {
                assert_eq!(name, "echo");
                assert!(!args.is_empty());
                // The argument should contain the arithmetic expansion
                assert!(args[0].contains("$(("));
            }
            _ => panic!("Expected Command node"),
        }
    }
}

#[test]
fn test_arithmetic_error_handling() {
    let mut interpreter = Interpreter::new();
    
    // Test division by zero
    let result = interpreter.execute("echo $((5 / 0))");
    assert!(result.is_ok()); // Should not crash, but may print error
    
    // Test invalid expression
    let result = interpreter.execute("(( invalid_expr ))");
    assert!(result.is_ok()); // Should not crash, but may print error
}

#[test]
fn test_arithmetic_nested_expressions() {
    let mut interpreter = Interpreter::new();
    
    // Test nested arithmetic
    let result = interpreter.execute("echo $((2 + $((3 * 4))))");
    assert!(result.is_ok());
    
    // Test arithmetic command with nested expansion
    let result = interpreter.execute("(( $((5 + 3)) > 7 ))");
    assert_eq!(result.unwrap(), 0); // Should be true
}

#[test]
fn test_arithmetic_in_loops() {
    let mut interpreter = Interpreter::new();
    
    // Test arithmetic in for loop
    let script = r#"
        for i in 1 2 3; do
            if (( i > 1 )); then
                echo "i is $i"
            fi
        done
    "#;
    
    let result = interpreter.execute(script);
    assert!(result.is_ok());
}

#[test]
fn test_arithmetic_whitespace_handling() {
    let mut interpreter = Interpreter::new();
    
    // Test various whitespace scenarios
    let test_cases = vec![
        "((5+3))",
        "(( 5 + 3 ))",
        "((  5  +  3  ))",
        "echo $((5+3))",
        "echo $(( 5 + 3 ))",
        "echo $((  5  +  3  ))",
    ];
    
    for command in test_cases {
        let result = interpreter.execute(command);
        assert!(result.is_ok(), "Failed to execute: {}", command);
    }
}
#[test]
fn debug_arithmetic() {
    let mut interpreter = Interpreter::new();
    
    // Set up variables
    interpreter.variables.insert("x".to_string(), "10".to_string());
    interpreter.variables.insert("y".to_string(), "5".to_string());
    
    // Test each case individually
    println!("Testing (( $x > $y ))");
    let result = interpreter.execute("(( $x > $y ))");
    println!("Result: {:?}", result);
    
    println!("Testing (( $y > $x ))");
    let result = interpreter.execute("(( $y > $x ))");
    println!("Result: {:?}", result);
    
    println!("Testing (( $x == 10 ))");
    let result = interpreter.execute("(( $x == 10 ))");
    println!("Result: {:?}", result);
    
    println!("Testing (( $y == 10 ))");
    let result = interpreter.execute("(( $y == 10 ))");
    println!("Result: {:?}", result);
}

#[test]
fn test_arithmetic_command_direct() {
    let mut interpreter = Interpreter::new();
    
    // Set up variables
    interpreter.variables.insert("y".to_string(), "5".to_string());
    
    // Test the expression that's failing
    println!("Testing expression: '5 == 10'");
    let result = interpreter.execute("(( 5 == 10 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 1); // Should be false (exit code 1)
    
    println!("Testing expression: '$y == 10'");
    let result = interpreter.execute("(( $y == 10 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 1); // Should be false (exit code 1)
    
    println!("Testing expression: '5 == 5'");
    let result = interpreter.execute("(( 5 == 5 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true (exit code 0)
}

#[test]
fn test_arithmetic_simple_cases() {
    let mut interpreter = Interpreter::new();
    
    // Test simple arithmetic without comparison
    println!("Testing: (( 5 ))");
    let result = interpreter.execute("(( 5 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Non-zero should be true (exit code 0)
    
    println!("Testing: (( 0 ))");
    let result = interpreter.execute("(( 0 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 1); // Zero should be false (exit code 1)
    
    // Test simple comparison
    println!("Testing: (( 5 > 3 ))");
    let result = interpreter.execute("(( 5 > 3 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // True
    
    println!("Testing: (( 3 > 5 ))");
    let result = interpreter.execute("(( 3 > 5 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 1); // False
}

#[test]
fn test_equality_operator() {
    let mut interpreter = Interpreter::new();
    
    // Test equality operator
    println!("Testing: (( 5 == 5 ))");
    let result = interpreter.execute("(( 5 == 5 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true (exit code 0)
    
    println!("Testing: (( 5 == 10 ))");
    let result = interpreter.execute("(( 5 == 10 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 1); // Should be false (exit code 1)
    
    println!("Testing: (( 10 == 10 ))");
    let result = interpreter.execute("(( 10 == 10 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true (exit code 0)
}

#[test]
fn test_nested_step_by_step() {
    let mut interpreter = Interpreter::new();
    
    // Test simple arithmetic expansion
    println!("Testing: echo $((5 + 3))");
    let result = interpreter.execute("echo $((5 + 3))");
    println!("Result: {:?}", result);
    assert!(result.is_ok());
    
    // Test simple arithmetic command
    println!("Testing: (( 8 > 7 ))");
    let result = interpreter.execute("(( 8 > 7 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true
    
    // Test the problematic nested case
    println!("Testing: (( $((5 + 3)) > 7 ))");
    let result = interpreter.execute("(( $((5 + 3)) > 7 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true
}

#[test]
fn test_gte_operator() {
    let mut interpreter = Interpreter::new();
    
    println!("Testing: (( 5 >= 5 ))");
    let result = interpreter.execute("(( 5 >= 5 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true
    
    println!("Testing: (( 3 >= 5 ))");
    let result = interpreter.execute("(( 3 >= 5 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 1); // Should be false
    
    println!("Testing: (( 7 >= 5 ))");
    let result = interpreter.execute("(( 7 >= 5 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true
}

#[test]
fn test_complex_nested_arithmetic() {
    let mut interpreter = Interpreter::new();
    
    // Test multiple nested arithmetic expressions
    println!("Testing: (( $((2 * 3)) + $((4 + 1)) == 11 ))");
    let result = interpreter.execute("(( $((2 * 3)) + $((4 + 1)) == 11 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true (6 + 5 == 11)
    
    // Test nested arithmetic with variables
    interpreter.variables.insert("a".to_string(), "3".to_string());
    interpreter.variables.insert("b".to_string(), "4".to_string());
    
    println!("Testing: (( $((a * 2)) > $((b + 1)) ))");
    let result = interpreter.execute("(( $((a * 2)) > $((b + 1)) ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true (6 > 5)
    
    // Test deeply nested arithmetic
    println!("Testing: (( $((1 + $((2 * 3)))) == 7 ))");
    let result = interpreter.execute("(( $((1 + $((2 * 3)))) == 7 ))");
    println!("Result: {:?}", result);
    assert_eq!(result.unwrap(), 0); // Should be true (1 + 6 == 7)
}

#[test]
fn debug_nested_with_variables() {
    let mut interpreter = Interpreter::new();
    
    interpreter.variables.insert("a".to_string(), "3".to_string());
    interpreter.variables.insert("b".to_string(), "4".to_string());
    
    // Test the individual parts
    println!("Testing: echo $((a * 2))");
    let result = interpreter.execute("echo $((a * 2))");
    println!("Result: {:?}", result);
    
    println!("Testing: echo $((b + 1))");
    let result = interpreter.execute("echo $((b + 1))");
    println!("Result: {:?}", result);
    
    println!("Testing: (( 6 > 5 ))");
    let result = interpreter.execute("(( 6 > 5 ))");
    println!("Result: {:?}", result);
    
    println!("Testing: (( $((a * 2)) > $((b + 1)) ))");
    let result = interpreter.execute("(( $((a * 2)) > $((b + 1)) ))");
    println!("Result: {:?}", result);
}

#[test]
fn debug_deeply_nested() {
    let mut interpreter = Interpreter::new();
    
    // Test the innermost expression first
    println!("Testing: echo $((2 * 3))");
    let result = interpreter.execute("echo $((2 * 3))");
    println!("Result: {:?}", result);
    
    // Test the outer expression with a literal value
    println!("Testing: echo $((1 + 6))");
    let result = interpreter.execute("echo $((1 + 6))");
    println!("Result: {:?}", result);
    
    // Test the nested expression
    println!("Testing: echo $((1 + $((2 * 3))))");
    let result = interpreter.execute("echo $((1 + $((2 * 3))))");
    println!("Result: {:?}", result);
    
    // Test in arithmetic command
    println!("Testing: (( 7 == 7 ))");
    let result = interpreter.execute("(( 7 == 7 ))");
    println!("Result: {:?}", result);
    
    // Test the full nested arithmetic command
    println!("Testing: (( $((1 + $((2 * 3)))) == 7 ))");
    let result = interpreter.execute("(( $((1 + $((2 * 3)))) == 7 ))");
    println!("Result: {:?}", result);
}
