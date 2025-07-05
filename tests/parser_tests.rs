/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use flash::lexer::Lexer;
use flash::parser::{Node, Parser, RedirectKind};

fn parse_script(script: &str) -> Node {
    let lexer = Lexer::new(script);
    let mut parser = Parser::new(lexer);
    parser.parse_script()
}

#[test]
fn test_parser_simple_command() {
    let ast = parse_script("echo hello");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Command { name, args, .. } => {
                    assert_eq!(name, "echo");
                    assert_eq!(args, &vec!["hello".to_string()]);
                }
                _ => panic!("Expected command node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_command_with_args() {
    let ast = parse_script("ls -la /home");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Command { name, args, .. } => {
                    assert_eq!(name, "ls");
                    assert_eq!(args, &vec!["-la".to_string(), "/home".to_string()]);
                }
                _ => panic!("Expected command node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_pipeline() {
    let ast = parse_script("ls | grep test");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Pipeline { commands } => {
                    assert_eq!(commands.len(), 2);
                }
                _ => panic!("Expected pipeline node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_if_statement() {
    let ast = parse_script("if [ -f file ]; then echo found; fi");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::IfStatement {
                    condition,
                    consequence,
                    ..
                } => {
                    // condition and consequence are Box<Node>, not Option
                    assert!(matches!(**condition, Node::Command { .. }));
                    assert!(matches!(**consequence, Node::List { .. }));
                }
                _ => panic!("Expected if statement node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_for_loop() {
    let ast = parse_script("for i in 1 2 3; do echo $i; done");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::ForLoop {
                    variable,
                    iterable,
                    body,
                } => {
                    assert_eq!(variable, "i");
                    match &**iterable {
                        Node::List { statements, .. } => {
                            assert_eq!(statements.len(), 3);
                        }
                        _ => panic!("Expected list node for iterable"),
                    }
                    assert!(matches!(**body, Node::List { .. }));
                }
                _ => panic!("Expected for loop node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_while_loop() {
    let ast = parse_script("while [ $i -lt 10 ]; do echo $i; done");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::WhileLoop { condition, body } => {
                    assert!(matches!(**condition, Node::Command { .. }));
                    assert!(matches!(**body, Node::List { .. }));
                }
                _ => panic!("Expected while loop node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_case_statement() {
    let ast = parse_script("case $var in pattern1) echo one ;; pattern2) echo two ;; esac");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::CaseStatement {
                    expression,
                    patterns,
                } => {
                    assert!(matches!(**expression, Node::StringLiteral(_)));
                    assert_eq!(patterns.len(), 2);
                }
                _ => panic!("Expected case statement node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_function_definition() {
    let ast = parse_script("function test_func() { echo hello; }");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Function { name, body } => {
                    assert_eq!(name, "test_func");
                    assert!(matches!(**body, Node::List { .. }));
                }
                _ => panic!("Expected function definition node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_output_redirection() {
    let ast = parse_script("echo hello > output.txt");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Command {
                    name,
                    args,
                    redirects,
                } => {
                    assert_eq!(name, "echo");
                    assert_eq!(args, &vec!["hello".to_string()]);
                    assert_eq!(redirects.len(), 1);
                    assert_eq!(redirects[0].kind, RedirectKind::Output);
                    assert_eq!(redirects[0].file, "output.txt");
                }
                _ => panic!("Expected command node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_append_redirection() {
    let ast = parse_script("echo hello >> output.txt");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Command {
                    name,
                    args,
                    redirects,
                } => {
                    assert_eq!(name, "echo");
                    assert_eq!(args, &vec!["hello".to_string()]);
                    assert_eq!(redirects.len(), 1);
                    assert_eq!(redirects[0].kind, RedirectKind::Append);
                    assert_eq!(redirects[0].file, "output.txt");
                }
                _ => panic!("Expected command node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_input_redirection() {
    let ast = parse_script("cat < input.txt");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Command {
                    name,
                    args,
                    redirects,
                } => {
                    assert_eq!(name, "cat");
                    assert_eq!(args.len(), 0);
                    assert_eq!(redirects.len(), 1);
                    assert_eq!(redirects[0].kind, RedirectKind::Input);
                    assert_eq!(redirects[0].file, "input.txt");
                }
                _ => panic!("Expected command node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_background_command() {
    let ast = parse_script("sleep 5 &");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::Command { name, args, .. } => {
                    assert_eq!(name, "sleep");
                    assert_eq!(args, &vec!["5".to_string()]);
                }
                _ => panic!("Expected command node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}

#[test]
fn test_parser_if_else_statement() {
    let ast = parse_script("if [ -f file ]; then echo found; else echo not found; fi");

    match ast {
        Node::List { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Node::IfStatement {
                    condition,
                    consequence,
                    alternative,
                } => {
                    assert!(matches!(**condition, Node::Command { .. }));
                    assert!(matches!(**consequence, Node::List { .. }));
                    assert!(alternative.is_some());
                }
                _ => panic!("Expected if statement node"),
            }
        }
        _ => panic!("Expected list node"),
    }
}
#[test]
fn test_parse_arithmetic_command() {
    use flash::lexer::Lexer;
    use flash::parser::{Node, Parser};

    let lexer = Lexer::new("(( 5 == 10 ))");
    let mut parser = Parser::new(lexer);

    let ast = parser.parse_script();
    println!("AST: {:?}", ast);

    // Check that we get an ArithmeticCommand node
    if let Node::List { statements, .. } = ast {
        assert_eq!(statements.len(), 1);
        if let Node::ArithmeticCommand { expression } = &statements[0] {
            println!("Expression: '{}'", expression);
            assert!(expression.contains("5"));
            assert!(expression.contains("10"));
            assert!(expression.contains("=="));
        } else {
            panic!("Expected ArithmeticCommand, got {:?}", statements[0]);
        }
    } else {
        panic!("Expected List, got {:?}", ast);
    }
}

#[test]
fn test_parse_equality_expression() {
    use flash::lexer::Lexer;
    use flash::parser::{Node, Parser};

    let lexer = Lexer::new("(( 5 == 10 ))");
    let mut parser = Parser::new(lexer);

    let ast = parser.parse_script();
    println!("AST: {:?}", ast);

    // Check the exact expression string
    if let Node::List { statements, .. } = ast {
        if let Node::ArithmeticCommand { expression } = &statements[0] {
            println!("Exact expression: '{}'", expression);
            // Check that it contains ==, not separate = characters
            assert!(expression.contains("=="));
            assert!(!expression.contains("= ="));
        }
    }
}

#[test]
fn test_parse_gte_expression() {
    use flash::lexer::Lexer;
    use flash::parser::{Node, Parser};

    let lexer = Lexer::new("(( 3 >= 5 ))");
    let mut parser = Parser::new(lexer);

    let ast = parser.parse_script();
    println!("AST: {:?}", ast);

    // Check the exact expression string
    if let Node::List { statements, .. } = ast {
        if let Node::ArithmeticCommand { expression } = &statements[0] {
            println!("Exact expression: '{}'", expression);
            // Check that it contains >=
            assert!(expression.contains(">="));
        }
    }
}

#[test]
fn test_parse_nested_arithmetic() {
    use flash::lexer::Lexer;
    use flash::parser::{Node, Parser};

    let lexer = Lexer::new("(( $((5 + 3)) > 7 ))");
    let mut parser = Parser::new(lexer);

    let ast = parser.parse_script();
    println!("AST: {:?}", ast);

    // Check what expression is being generated
    if let Node::List { statements, .. } = ast {
        println!("Number of statements: {}", statements.len());
        for (i, stmt) in statements.iter().enumerate() {
            println!("Statement {}: {:?}", i, stmt);
        }
    }
}

#[test]
fn test_parse_arithmetic_expansion_with_vars() {
    use flash::lexer::Lexer;
    use flash::parser::Parser;

    let lexer = Lexer::new("echo $((a * 2))");
    let mut parser = Parser::new(lexer);

    let ast = parser.parse_script();
    println!("AST: {:?}", ast);
}

#[test]
fn test_parse_deeply_nested() {
    use flash::lexer::Lexer;
    use flash::parser::{Node, Parser};

    let lexer = Lexer::new("(( $((1 + $((2 * 3)))) == 7 ))");
    let mut parser = Parser::new(lexer);

    let ast = parser.parse_script();
    println!("AST: {:?}", ast);

    // Check what expression is being generated
    if let Node::List { statements, .. } = ast {
        println!("Number of statements: {}", statements.len());
        for (i, stmt) in statements.iter().enumerate() {
            println!("Statement {}: {:?}", i, stmt);
        }
    }
}
