/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use flash::lexer::{Lexer, TokenKind};

#[test]
fn test_lexer_basic_tokens() {
    let mut lexer = Lexer::new("echo hello");
    
    let token1 = lexer.next_token();
    assert_eq!(token1.kind, TokenKind::Word("echo".to_string()));
    
    let token2 = lexer.next_token();
    assert_eq!(token2.kind, TokenKind::Word("hello".to_string()));
    
    let token3 = lexer.next_token();
    assert_eq!(token3.kind, TokenKind::EOF);
}

#[test]
fn test_lexer_operators() {
    let mut lexer = Lexer::new("| && || ; & < > >>");
    
    assert_eq!(lexer.next_token().kind, TokenKind::Pipe);
    assert_eq!(lexer.next_token().kind, TokenKind::And);
    assert_eq!(lexer.next_token().kind, TokenKind::Or);
    assert_eq!(lexer.next_token().kind, TokenKind::Semicolon);
    assert_eq!(lexer.next_token().kind, TokenKind::Background);
    assert_eq!(lexer.next_token().kind, TokenKind::Less);
    assert_eq!(lexer.next_token().kind, TokenKind::Great);
    assert_eq!(lexer.next_token().kind, TokenKind::DGreat);
}

#[test]
fn test_lexer_quotes() {
    let mut lexer = Lexer::new(r#""hello world" 'single quoted'"#);
    
    assert_eq!(lexer.next_token().kind, TokenKind::Quote);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("hello world".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Quote);
    assert_eq!(lexer.next_token().kind, TokenKind::SingleQuote);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("single quoted".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::SingleQuote);
}

#[test]
fn test_lexer_variable_expansion() {
    let mut lexer = Lexer::new("$HOME ${USER} $1 $@");
    
    assert_eq!(lexer.next_token().kind, TokenKind::Dollar);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("HOME".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Dollar);
    assert_eq!(lexer.next_token().kind, TokenKind::LBrace);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("USER".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RBrace);
    assert_eq!(lexer.next_token().kind, TokenKind::Dollar);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("1".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Dollar);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("@".to_string()));
}

#[test]
fn test_lexer_command_substitution() {
    let mut lexer = Lexer::new("$(echo hello) `date`");
    
    assert_eq!(lexer.next_token().kind, TokenKind::CmdSubst);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("hello".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
    assert_eq!(lexer.next_token().kind, TokenKind::Backtick);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("date".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Backtick);
}

#[test]
fn test_lexer_arithmetic_expansion() {
    let mut lexer = Lexer::new("$((1 + 2))");
    
    assert_eq!(lexer.next_token().kind, TokenKind::ArithSubst);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("1".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("+".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("2".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
}

#[test]
fn test_lexer_keywords() {
    let mut lexer = Lexer::new("if then elif else fi for while until do done function case esac");
    
    assert_eq!(lexer.next_token().kind, TokenKind::If);
    assert_eq!(lexer.next_token().kind, TokenKind::Then);
    assert_eq!(lexer.next_token().kind, TokenKind::Elif);
    assert_eq!(lexer.next_token().kind, TokenKind::Else);
    assert_eq!(lexer.next_token().kind, TokenKind::Fi);
    assert_eq!(lexer.next_token().kind, TokenKind::For);
    assert_eq!(lexer.next_token().kind, TokenKind::While);
    assert_eq!(lexer.next_token().kind, TokenKind::Until);
    assert_eq!(lexer.next_token().kind, TokenKind::Do);
    assert_eq!(lexer.next_token().kind, TokenKind::Done);
    assert_eq!(lexer.next_token().kind, TokenKind::Function);
    assert_eq!(lexer.next_token().kind, TokenKind::Case);
    assert_eq!(lexer.next_token().kind, TokenKind::Esac);
}

#[test]
fn test_lexer_comments() {
    let mut lexer = Lexer::new("echo hello # this is a comment\necho world");
    
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("hello".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Comment);
    assert_eq!(lexer.next_token().kind, TokenKind::Newline);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("world".to_string()));
}

#[test]
fn test_lexer_newlines() {
    let mut lexer = Lexer::new("echo\n\nworld");
    
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Newline);
    assert_eq!(lexer.next_token().kind, TokenKind::Newline);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("world".to_string()));
}

#[test]
fn test_lexer_braces() {
    let mut lexer = Lexer::new("{ echo hello; }");
    
    assert_eq!(lexer.next_token().kind, TokenKind::LBrace);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("hello".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Semicolon);
    assert_eq!(lexer.next_token().kind, TokenKind::RBrace);
}

#[test]
fn test_lexer_parentheses() {
    let mut lexer = Lexer::new("(echo hello)");
    
    assert_eq!(lexer.next_token().kind, TokenKind::LParen);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("hello".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
}

#[test]
fn test_lexer_assignment() {
    let mut lexer = Lexer::new("VAR=value");
    
    assert_eq!(lexer.next_token().kind, TokenKind::Word("VAR".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Assignment);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("value".to_string()));
}

#[test]
fn test_lexer_extended_glob() {
    let mut lexer = Lexer::new("?(pattern) *(pattern) +(pattern) @(pattern) !(pattern)");
    
    assert_eq!(lexer.next_token().kind, TokenKind::ExtGlob('?'));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("pattern".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
    
    assert_eq!(lexer.next_token().kind, TokenKind::ExtGlob('*'));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("pattern".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
    
    assert_eq!(lexer.next_token().kind, TokenKind::ExtGlob('+'));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("pattern".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
    
    assert_eq!(lexer.next_token().kind, TokenKind::ExtGlob('@'));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("pattern".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
    
    assert_eq!(lexer.next_token().kind, TokenKind::ExtGlob('!'));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("pattern".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
}

#[test]
fn test_lexer_double_semicolon() {
    let mut lexer = Lexer::new("case $var in pattern) echo hello ;; esac");
    
    assert_eq!(lexer.next_token().kind, TokenKind::Case);
    assert_eq!(lexer.next_token().kind, TokenKind::Dollar);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("var".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::In);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("pattern".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RParen);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("hello".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::DoubleSemicolon);
    assert_eq!(lexer.next_token().kind, TokenKind::Esac);
}

#[test]
fn test_lexer_position_tracking() {
    let mut lexer = Lexer::new("echo\nhello");
    
    let token1 = lexer.next_token();
    assert_eq!(token1.position.line, 1);
    assert_eq!(token1.position.column, 1);
    
    let token2 = lexer.next_token(); // newline
    assert_eq!(token2.position.line, 1);
    assert_eq!(token2.position.column, 5);
    
    let token3 = lexer.next_token();
    assert_eq!(token3.position.line, 2);
    assert_eq!(token3.position.column, 1);
}

#[test]
fn test_lexer_whitespace_handling() {
    let mut lexer = Lexer::new("  echo   hello  ");
    
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("hello".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::EOF);
}

#[test]
fn test_lexer_empty_input() {
    let mut lexer = Lexer::new("");
    assert_eq!(lexer.next_token().kind, TokenKind::EOF);
}

#[test]
fn test_lexer_only_whitespace() {
    let mut lexer = Lexer::new("   \t  \n  ");
    assert_eq!(lexer.next_token().kind, TokenKind::Newline);
    assert_eq!(lexer.next_token().kind, TokenKind::EOF);
}

#[test]
fn test_lexer_mixed_quotes() {
    let mut lexer = Lexer::new(r#"echo "hello 'world'" 'goodbye "friend"'"#);
    
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Quote);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("hello 'world'".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Quote);
    assert_eq!(lexer.next_token().kind, TokenKind::SingleQuote);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("goodbye \"friend\"".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::SingleQuote);
}

#[test]
fn test_lexer_escape_sequences() {
    let mut lexer = Lexer::new(r#"echo \$HOME \n \t \\"#);
    
    assert_eq!(lexer.next_token().kind, TokenKind::Word("echo".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("\\$HOME".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("\\n".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("\\t".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("\\\\".to_string()));
}

#[test]
fn test_lexer_complex_redirection() {
    let mut lexer = Lexer::new("cmd 2>&1 3< file 4>> log");
    
    assert_eq!(lexer.next_token().kind, TokenKind::Word("cmd".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("2".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Great);
    assert_eq!(lexer.next_token().kind, TokenKind::Background);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("1".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("3".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Less);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("file".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("4".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::DGreat);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("log".to_string()));
}

#[test]
fn test_lexer_glob_patterns() {
    let mut lexer = Lexer::new("ls *.txt [abc] {1,2,3}");
    
    assert_eq!(lexer.next_token().kind, TokenKind::Word("ls".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("*.txt".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::Word("[abc]".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::LBrace);
    assert_eq!(lexer.next_token().kind, TokenKind::Word("1,2,3".to_string()));
    assert_eq!(lexer.next_token().kind, TokenKind::RBrace);
}