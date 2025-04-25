pub mod formatter;
pub mod interpreter;
pub mod lexer;
pub mod parser;

// fn main() -> io::Result<()> {
//     let mut interpreter = Interpreter::new();
//     interpreter.run_interactive()?;
//     Ok(())
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_lexer() {
//         let input = "echo hello | grep world";
//         let mut lexer = Lexer::new(input);

//         let expected_tokens = vec![
//             TokenKind::Word("echo".to_string()),
//             TokenKind::Word("hello".to_string()),
//             TokenKind::Pipe,
//             TokenKind::Word("grep".to_string()),
//             TokenKind::Word("world".to_string()),
//             TokenKind::EOF,
//         ];

//         for expected in expected_tokens {
//             let token = lexer.next_token();
//             assert_eq!(token.kind, expected);
//         }
//     }

//     #[test]
//     fn test_parser() {
//         let input = "echo hello > output.txt";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);

//         if let Node::Command {
//             name,
//             args,
//             redirects,
//         } = parser.parse_command()
//         {
//             assert_eq!(name, "echo");
//             assert_eq!(args, vec!["hello"]);
//             assert_eq!(redirects.len(), 1);
//             assert!(matches!(redirects[0].kind, RedirectKind::Output));
//             assert_eq!(redirects[0].file, "output.txt");
//         } else {
//             panic!("Expected Command node");
//         }
//     }

//     #[test]
//     fn test_formatter() {
//         let input = "if [ -f /etc/bashrc ]; then\nsource /etc/bashrc\nfi";
//         let formatted = format_script_with_options(input, " ", 0);

//         // This is a simplified test. In a real formatter, this would actually parse the if/then/fi
//         // constructs correctly
//         assert!(formatted.contains("source /etc/bashrc"));
//     }

//     #[test]
//     fn test_variable_expansion() {
//         let mut interpreter = Interpreter::new();
//         interpreter
//             .variables
//             .insert("NAME".to_string(), "world".to_string());

//         let expanded = interpreter.expand_variables("Hello $NAME!");
//         assert_eq!(expanded, "Hello world!");

//         let expanded = interpreter.expand_variables("Hello ${NAME}!");
//         assert_eq!(expanded, "Hello world!");
//     }

//     #[test]
//     fn test_command_execution() {
//         let mut interpreter = Interpreter::new();

//         // Test a basic command
//         let result = interpreter.execute("echo test").unwrap();
//         assert_eq!(result, 0);

//         // Test assignment
//         let result = interpreter.execute("X=test").unwrap();
//         assert_eq!(result, 0);
//         assert_eq!(interpreter.variables.get("X"), Some(&"test".to_string()));
//     }

//     // #[test]
//     // fn test_pipeline() {
//     //     let input = "echo hello | grep e";
//     //     let lexer = Lexer::new(input);
//     //     let mut parser = Parser::new(lexer);

//     //     if let Node::Pipeline { commands } = parser.parse_command() {
//     //         assert_eq!(commands.len(), 2);

//     //         if let Node::Command { name, args, .. } = &commands[0] {
//     //             assert_eq!(name, "echo");
//     //             assert_eq!(args, &["hello"]);
//     //         } else {
//     //             panic!("Expected Command node");
//     //         }

//     //         if let Node::Command { name, args, .. } = &commands[1] {
//     //             assert_eq!(name, "grep");
//     //             assert_eq!(args, &["e"]);
//     //         } else {
//     //             panic!("Expected Command node");
//     //         }
//     //     } else {
//     //         panic!("Expected Pipeline node");
//     //     }
//     // }

//     #[test]
//     fn test_complex_pipeline() {
//         let input = "cat file.txt | grep pattern | sort | uniq -c | sort -nr";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);

//         if let Node::Pipeline { commands } = parser.parse_command() {
//             assert_eq!(commands.len(), 5);

//             if let Node::Command { name, .. } = &commands[0] {
//                 assert_eq!(name, "cat");
//             }

//             if let Node::Command { name, .. } = &commands[4] {
//                 assert_eq!(name, "sort");
//             }
//         } else {
//             panic!("Expected Pipeline node");
//         }
//     }

//     #[test]
//     fn test_subshell() {
//         let input = "(cd /tmp && ls)";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);

//         if let Node::Subshell { list } = parser.parse_statement().unwrap() {
//             if let Node::List {
//                 statements,
//                 operators,
//             } = list.as_ref()
//             {
//                 assert_eq!(statements.len(), 2);
//                 assert_eq!(operators, &["&&".to_string()]);
//             } else {
//                 panic!("Expected List node");
//             }
//         } else {
//             panic!("Expected Subshell node");
//         }
//     }

//     #[test]
//     fn test_logical_operators() {
//         let input = "true && echo success || echo failure";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);
//         let node = parser.parse_script();

//         if let Node::List {
//             statements,
//             operators,
//         } = node
//         {
//             assert_eq!(statements.len(), 3);
//             assert_eq!(operators, &["&&".to_string(), "||".to_string()]);
//         } else {
//             panic!("Expected List node");
//         }
//     }

//     #[test]
//     fn test_comments() {
//         let input = "echo hello # this is a comment\necho world";
//         let lexer = Lexer::new(input);
//         let mut parser = Parser::new(lexer);
//         let node = parser.parse_script();

//         if let Node::List { statements, .. } = node {
//             assert_eq!(statements.len(), 3); // echo hello, comment, echo world

//             match &statements[1] {
//                 Node::Comment(text) => {
//                     assert!(text.starts_with("# this is a comment"));
//                 }
//                 _ => panic!("Expected Comment node"),
//             }
//         } else {
//             panic!("Expected List node");
//         }
//     }

// //     #[test]
// //     fn integration_test_basic_script_with_variable_and_if_and_else() {
// //         let script = r#"
// //     #!/bin/bash
// //     # This is a test script
// //     echo "Starting test"
// //     RESULT=$(echo "test" | grep "t")
// //     echo "Result: $RESULT"
// //     if [ -f "/tmp/test" ]; then
// //         echo "File exists"
// //     else
// //         echo "File doesn't exist"
// //     fi
// //     "#;

// //         let mut interpreter = Interpreter::new();
// //         let result = interpreter.execute(script).unwrap();

// //         // Just make sure it runs without errors
// //         assert_eq!(result, 0);
// //     }

// //     #[test]
// //     fn integration_test_basic_script() {
// //         let script = r#"
// // # Simple test script with basic commands only
// // echo "Starting test"
// // MESSAGE="Hello world"
// // echo "Message: $MESSAGE"
// // cd /tmp
// // echo "Current directory: $(pwd)"
// // "#;

// //         let mut interpreter = Interpreter::new();
// //         let result = interpreter.execute(script).unwrap();

// //         // Just make sure it runs without errors
// //         assert_eq!(result, 0);
// //     }

//     #[test]
//     fn integration_test_formatter() {
//         let script = "if [ $x -eq 42 ]; then echo \"The answer\"; fi";
//         let formatted = format_script_with_options(script, "  ", 0);

//         println!("{:?}", formatted);

//         // Check that the formatter adds appropriate whitespace
//         assert!(formatted.contains("if [ $x -eq 42 ]"));
//         assert!(formatted.contains("echo \"The answer\""));
//     }
// }

// // Example usage of the library components

// fn example_usage() {
//     // Example 1: Parse and execute a simple script
//     let script = "echo Hello, world!";
//     let mut interpreter = Interpreter::new();
//     let exit_code = interpreter.execute(script).unwrap();
//     println!("Script executed with exit code: {}", exit_code);

//     // Example 2: Format a script
//     let script = "if [ $x -eq 42 ]; then echo \"The answer\"; fi";
//     let formatted = format_script_with_options(script, "  ", 0);
//     println!("Formatted script:\n{}", formatted);

//     // Example 3: Lexer and parser usage
//     let script = "echo $HOME | grep '/home'";
//     let lexer = Lexer::new(script);
//     let mut parser = Parser::new(lexer);
//     let ast = parser.parse_script();

//     // We could implement a proper Debug implementation to print the AST
//     println!("AST: {:?}", ast);
// }

// // Utility functions

// /// Parse a shell script and return its AST
// fn parse_script(script: &str) -> Node {
//     let lexer = Lexer::new(script);
//     let mut parser = Parser::new(lexer);
//     parser.parse_script()
// }

// /// Execute a shell script and return the exit code
// fn execute_script(script: &str) -> Result<i32, io::Error> {
//     let mut interpreter = Interpreter::new();
//     interpreter.execute(script)
// }

// /// Format a shell script with the specified indentation
// fn format_script_with_options(script: &str, indent: &str, indent_level: usize) -> String {
//     let ast = parse_script(script);
//     let mut formatter = Formatter::new(indent);
//     formatter.set_indent_level(indent_level);
//     formatter.format(&ast)
// }
