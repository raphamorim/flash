/// Token types that can be produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Word(String),
    Assignment,    // =
    Pipe,          // |
    Semicolon,     // ;
    Newline,       // \n
    And,           // &&
    Background,    // & (add this new token)
    Or,            // ||
    LParen,        // (
    RParen,        // )
    LBrace,        // {
    RBrace,        // }
    Less,          // <
    Great,         // >
    DGreat,        // >>
    Dollar,        // $
    Quote,         // "
    SingleQuote,   // '
    Backtick,      // `
    Comment,       // #
    CmdSubst,      // $(
    ExtGlob(char), // For ?(, *(, +(, @(, !(
    // New token types for shell control flow
    If,   // if keyword
    Then, // then keyword
    Elif, // elif keyword
    Else, // else keyword
    Fi,   // fi keyword
    EOF,
}

/// A token produced by the lexer
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub position: Position,
}

/// Source position information
#[derive(Debug, Clone, Copy)]
pub struct Position {
    line: usize,
    column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

fn is_special_char(ch: char) -> bool {
    match ch {
        '=' | '|' | ';' | '\n' | '&' | '(' | ')' | '{' | '}' | '<' | '>' | '$' | '"' | '\''
        | '`' | '#' => true,
        // Removed '?', '*', '+', '@', '!' to allow them in normal words
        _ => false,
    }
}

/// Lexer that converts input text into tokens
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    read_position: usize,
    ch: char,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let mut lexer = Self {
            input: input.chars().collect(),
            position: 0,
            read_position: 0,
            ch: '\0',
            line: 1,
            column: 0,
        };
        lexer.read_char();
        lexer
    }

    fn read_char(&mut self) {
        if self.read_position >= self.input.len() {
            self.ch = '\0';
        } else {
            self.ch = self.input[self.read_position];
        }
        self.position = self.read_position;
        self.read_position += 1;
        self.column += 1;
    }

    fn peek_char(&self) -> char {
        if self.read_position >= self.input.len() {
            '\0'
        } else {
            self.input[self.read_position]
        }
    }

    // Helper to check if the current position is followed by whitespace or a special character
    fn is_word_boundary(&self) -> bool {
        let peek = self.peek_char();
        peek.is_whitespace() || is_special_char(peek) || peek == '\0'
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let current_position = Position::new(self.line, self.column);

        let token = match self.ch {
            '=' => Token {
                kind: TokenKind::Assignment,
                value: "=".to_string(),
                position: current_position,
            },
            '|' => {
                if self.peek_char() == '|' {
                    self.read_char();
                    Token {
                        kind: TokenKind::Or,
                        value: "||".to_string(),
                        position: current_position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Pipe,
                        value: "|".to_string(),
                        position: current_position,
                    }
                }
            }
            ';' => Token {
                kind: TokenKind::Semicolon,
                value: ";".to_string(),
                position: current_position,
            },
            '&' => {
                if self.peek_char() == '&' {
                    self.read_char();
                    Token {
                        kind: TokenKind::And,
                        value: "&&".to_string(),
                        position: current_position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Background,
                        value: "&".to_string(),
                        position: current_position,
                    }
                }
            }
            '\n' => {
                self.line += 1;
                self.column = 0;
                Token {
                    kind: TokenKind::Newline,
                    value: "\n".to_string(),
                    position: current_position,
                }
            }
            '(' => Token {
                kind: TokenKind::LParen,
                value: "(".to_string(),
                position: current_position,
            },
            ')' => Token {
                kind: TokenKind::RParen,
                value: ")".to_string(),
                position: current_position,
            },
            '{' => Token {
                kind: TokenKind::LBrace,
                value: "{".to_string(),
                position: current_position,
            },
            '}' => Token {
                kind: TokenKind::RBrace,
                value: "}".to_string(),
                position: current_position,
            },
            '<' => Token {
                kind: TokenKind::Less,
                value: "<".to_string(),
                position: current_position,
            },
            '>' => {
                if self.peek_char() == '>' {
                    self.read_char();
                    Token {
                        kind: TokenKind::DGreat,
                        value: ">>".to_string(),
                        position: current_position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Great,
                        value: ">".to_string(),
                        position: current_position,
                    }
                }
            }
            '$' => {
                // Check for command substitution $( syntax
                if self.peek_char() == '(' {
                    self.read_char(); // Consume the '('
                    Token {
                        kind: TokenKind::CmdSubst,
                        value: "$(".to_string(),
                        position: current_position,
                    }
                } else {
                    Token {
                        kind: TokenKind::Dollar,
                        value: "$".to_string(),
                        position: current_position,
                    }
                }
            }
            '"' => Token {
                kind: TokenKind::Quote,
                value: "\"".to_string(),
                position: current_position,
            },
            '\'' => Token {
                kind: TokenKind::SingleQuote,
                value: "'".to_string(),
                position: current_position,
            },
            '`' => Token {
                kind: TokenKind::Backtick,
                value: "`".to_string(),
                position: current_position,
            },
            '#' => self.read_comment(),
            '\0' => Token {
                kind: TokenKind::EOF,
                value: "".to_string(),
                position: current_position,
            },
            'i' => {
                // Check for "if" keyword
                if self.peek_char() == 'f' && self.position + 1 < self.input.len() {
                    self.read_char(); // Consume 'f'
                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::If,
                            value: "if".to_string(),
                            position: current_position,
                        }
                    } else {
                        // If it's not a standalone "if", backtrack and treat as a word
                        self.position -= 1;
                        self.read_position -= 1;
                        self.column -= 1;
                        self.ch = 'i';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            't' => {
                // Check for "then" keyword
                if self.peek_char() == 'h'
                    && self.position + 3 < self.input.len()
                    && self.input[self.position + 1] == 'h'
                    && self.input[self.position + 2] == 'e'
                    && self.input[self.position + 3] == 'n'
                {
                    self.read_char(); // 'h'
                    self.read_char(); // 'e'
                    self.read_char(); // 'n'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::Then,
                            value: "then".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "then", backtrack and treat as a word
                        self.position -= 3;
                        self.read_position -= 3;
                        self.column -= 3;
                        self.ch = 't';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            'e' => {
                // Check for "else" or "elif" keywords
                if self.peek_char() == 'l' && self.position + 3 < self.input.len() {
                    self.read_char(); // 'l'

                    if self.peek_char() == 's' {
                        self.read_char(); // 's'
                        if self.peek_char() == 'e' {
                            self.read_char(); // 'e'
                            if self.is_word_boundary() {
                                Token {
                                    kind: TokenKind::Else,
                                    value: "else".to_string(),
                                    position: current_position,
                                }
                            } else {
                                // Not a standalone "else", backtrack
                                self.position -= 3;
                                self.read_position -= 3;
                                self.column -= 3;
                                self.ch = 'e';
                                self.read_word()
                            }
                        } else {
                            // Not "else", backtrack
                            self.position -= 2;
                            self.read_position -= 2;
                            self.column -= 2;
                            self.ch = 'e';
                            self.read_word()
                        }
                    } else if self.peek_char() == 'i' {
                        self.read_char(); // 'i'
                        if self.peek_char() == 'f' {
                            self.read_char(); // 'f'
                            if self.is_word_boundary() {
                                Token {
                                    kind: TokenKind::Elif,
                                    value: "elif".to_string(),
                                    position: current_position,
                                }
                            } else {
                                // Not a standalone "elif", backtrack
                                self.position -= 3;
                                self.read_position -= 3;
                                self.column -= 3;
                                self.ch = 'e';
                                self.read_word()
                            }
                        } else {
                            // Not "elif", backtrack
                            self.position -= 2;
                            self.read_position -= 2;
                            self.column -= 2;
                            self.ch = 'e';
                            self.read_word()
                        }
                    } else {
                        // Not "else" or "elif", backtrack
                        self.position -= 1;
                        self.read_position -= 1;
                        self.column -= 1;
                        self.ch = 'e';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            'f' => {
                // Check for "fi" keyword
                if self.peek_char() == 'i' && self.position + 1 < self.input.len() {
                    self.read_char(); // Consume 'i'
                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::Fi,
                            value: "fi".to_string(),
                            position: current_position,
                        }
                    } else {
                        // If it's not a standalone "fi", backtrack and treat as a word
                        self.position -= 1;
                        self.read_position -= 1;
                        self.column -= 1;
                        self.ch = 'f';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            _ => self.read_word(),
        };

        if token.kind != TokenKind::Word(String::new()) {
            self.read_char();
        }

        token
    }

    fn read_word(&mut self) -> Token {
        let position = Position::new(self.line, self.column);
        let mut word = String::new();

        // Check for extglob pattern prefixes
        if (self.ch == '?' || self.ch == '*' || self.ch == '+' || self.ch == '@' || self.ch == '!')
            && self.peek_char() == '('
        {
            let peek = self.peek_char();
            if peek == '(' {
                // This is an extglob pattern
                word.push(self.ch); // Add the operator

                self.read_char(); // Move past the operator
                word.push(self.ch); // Add the open paren
                self.read_char(); // Move past the open paren

                // Read until matching closing paren, accounting for nesting
                let mut depth = 1;

                while depth > 0 && self.ch != '\0' {
                    if self.ch == '(' {
                        depth += 1;
                    } else if self.ch == ')' {
                        depth -= 1;
                    }

                    word.push(self.ch);
                    self.read_char();
                }

                // After finding the closing parenthesis, continue reading
                // any suffixes (like ".txt") that should be part of the pattern
                while !self.ch.is_whitespace() && self.ch != '\0' && !is_special_char(self.ch) {
                    word.push(self.ch);
                    self.read_char();
                }

                return Token {
                    kind: TokenKind::Word(word.clone()),
                    value: word,
                    position,
                };
            }
        }

        // Normal word handling
        while !self.ch.is_whitespace() && self.ch != '\0' && !is_special_char(self.ch) {
            word.push(self.ch);
            self.read_char();
        }

        // For normal words including glob patterns
        while !self.ch.is_whitespace()
            && self.ch != '\0'
            && (self.ch == '*'
                || self.ch == '?'
                || self.ch == '['
                || self.ch == ']'
                || !is_special_char(self.ch))
        {
            word.push(self.ch);
            self.read_char();
        }

        // We moved ahead one character, so step back
        if self.position > 0 {
            self.position -= 1;
            self.read_position -= 1;
            self.column -= 1;
        }

        // Check for keywords after reading the full word
        let token_kind = match word.as_str() {
            "if" => TokenKind::If,
            "then" => TokenKind::Then,
            "elif" => TokenKind::Elif,
            "else" => TokenKind::Else,
            "fi" => TokenKind::Fi,
            _ => TokenKind::Word(word.clone()),
        };

        Token {
            kind: token_kind,
            value: word,
            position,
        }
    }

    fn read_comment(&mut self) -> Token {
        let position = Position::new(self.line, self.column);
        let mut comment = String::from("#");

        self.read_char(); // Skip the '#'

        while self.ch != '\n' && self.ch != '\0' {
            comment.push(self.ch);
            self.read_char();
        }

        // We moved ahead one character, so step back
        if self.position > 0 {
            self.position -= 1;
            self.read_position -= 1;
            self.column -= 1;
        }

        Token {
            kind: TokenKind::Comment,
            value: comment,
            position,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.ch.is_whitespace() && self.ch != '\n' {
            self.read_char();
        }
    }
}

#[cfg(test)]
mod lexer_tests {
    use crate::lexer::Lexer;
    use crate::lexer::TokenKind;

    #[test]
    fn debug_lexer_output() {
        let input = r#"LOG_DIR="/var/log""#;
        let mut lexer = Lexer::new(input);

        println!("Tokens for 'LOG_DIR=\"/var/log\"':");

        let mut token = lexer.next_token();
        while token.kind != TokenKind::EOF {
            println!("Token: {:?}", token);
            token = lexer.next_token();
        }
    }

    fn test_tokens(input: &str, expected_tokens: Vec<TokenKind>) {
        let mut lexer = Lexer::new(input);
        for expected in expected_tokens {
            let token = lexer.next_token();
            assert_eq!(
                token.kind, expected,
                "Expected {:?} but got {:?} for input: {}",
                expected, token.kind, input
            );
        }

        // Ensure we've consumed all tokens
        let final_token = lexer.next_token();
        assert_eq!(
            final_token.kind,
            TokenKind::EOF,
            "Expected EOF but got {:?}",
            final_token.kind
        );
    }

    #[test]
    fn test_basic_tokens() {
        let input = "ls -l | grep file";
        let expected = vec![
            TokenKind::Word("ls".to_string()),
            TokenKind::Word("-l".to_string()),
            TokenKind::Pipe,
            TokenKind::Word("grep".to_string()),
            TokenKind::Word("file".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_assignment() {
        let input = "VAR=value";
        let expected = vec![
            TokenKind::Word("VAR".to_string()),
            TokenKind::Assignment,
            TokenKind::Word("value".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_redirections() {
        let input = "ls > output.txt 2>&1";
        let expected = vec![
            TokenKind::Word("ls".to_string()),
            TokenKind::Great,
            TokenKind::Word("output.txt".to_string()),
            TokenKind::Word("2".to_string()),
            TokenKind::Great,
            TokenKind::Word("&1".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_quoted_strings() {
        let input = r#"echo "hello world" 'single quoted'"#;
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Quote,
            TokenKind::Word("hello".to_string()),
            TokenKind::Word("world".to_string()),
            TokenKind::Quote,
            TokenKind::SingleQuote,
            TokenKind::Word("single".to_string()),
            TokenKind::Word("quoted".to_string()),
            TokenKind::SingleQuote,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_command_substitution() {
        let input = "echo $(ls -l)";
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::CmdSubst,
            TokenKind::Word("ls".to_string()),
            TokenKind::Word("-l".to_string()),
            TokenKind::RParen,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_variable_expansion() {
        let input = "echo $HOME";
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("HOME".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_operators() {
        let input = "cmd1 && cmd2 || cmd3";
        let expected = vec![
            TokenKind::Word("cmd1".to_string()),
            TokenKind::And,
            TokenKind::Word("cmd2".to_string()),
            TokenKind::Or,
            TokenKind::Word("cmd3".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_background_process() {
        let input = "sleep 10 &";
        let expected = vec![
            TokenKind::Word("sleep".to_string()),
            TokenKind::Word("10".to_string()),
            TokenKind::Background,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_comments() {
        let input = "echo hello # this is a comment";
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("hello".to_string()),
            TokenKind::Comment,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_newlines() {
        let input = "cmd1\ncmd2\ncmd3";
        let expected = vec![
            TokenKind::Word("cmd1".to_string()),
            TokenKind::Newline,
            TokenKind::Word("cmd2".to_string()),
            TokenKind::Newline,
            TokenKind::Word("cmd3".to_string()),
        ];
        test_tokens(input, expected);
    }

    // Tests for shell control flow

    #[test]
    fn test_if_statement() {
        let input = "if test -f file.txt; then echo found; fi";
        let expected = vec![
            TokenKind::If,
            TokenKind::Word("test".to_string()),
            TokenKind::Word("-f".to_string()),
            TokenKind::Word("file.txt".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("found".to_string()),
            TokenKind::Semicolon,
            TokenKind::Fi,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_if_with_newlines() {
        let input = "if true\nthen\necho yes\nfi";
        let expected = vec![
            TokenKind::If,
            TokenKind::Word("true".to_string()),
            TokenKind::Newline,
            TokenKind::Then,
            TokenKind::Newline,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("yes".to_string()),
            TokenKind::Newline,
            TokenKind::Fi,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_if_else_statement() {
        let input = "if [ $a -eq 5 ]; then echo equal; else echo not equal; fi";
        let expected = vec![
            TokenKind::If,
            TokenKind::Word("[".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("a".to_string()),
            TokenKind::Word("-eq".to_string()),
            TokenKind::Word("5".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("equal".to_string()),
            TokenKind::Semicolon,
            TokenKind::Else,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("not".to_string()),
            TokenKind::Word("equal".to_string()),
            TokenKind::Semicolon,
            TokenKind::Fi,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_if_elif_else_statement() {
        let input =
            "if [ $a -eq 1 ]; then echo one; elif [ $a -eq 2 ]; then echo two; else echo other; fi";
        let expected = vec![
            TokenKind::If,
            TokenKind::Word("[".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("a".to_string()),
            TokenKind::Word("-eq".to_string()),
            TokenKind::Word("1".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("one".to_string()),
            TokenKind::Semicolon,
            TokenKind::Elif,
            TokenKind::Word("[".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("a".to_string()),
            TokenKind::Word("-eq".to_string()),
            TokenKind::Word("2".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("two".to_string()),
            TokenKind::Semicolon,
            TokenKind::Else,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("other".to_string()),
            TokenKind::Semicolon,
            TokenKind::Fi,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_nested_if_statements() {
        let input = "if true; then if false; then echo nested; fi; fi";
        let expected = vec![
            TokenKind::If,
            TokenKind::Word("true".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::If,
            TokenKind::Word("false".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("nested".to_string()),
            TokenKind::Semicolon,
            TokenKind::Fi,
            TokenKind::Semicolon,
            TokenKind::Fi,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_if_with_complex_command() {
        let input = "if grep -q pattern file.txt; then echo found; fi";
        let expected = vec![
            TokenKind::If,
            TokenKind::Word("grep".to_string()),
            TokenKind::Word("-q".to_string()),
            TokenKind::Word("pattern".to_string()),
            TokenKind::Word("file.txt".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("found".to_string()),
            TokenKind::Semicolon,
            TokenKind::Fi,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_control_flow_keywords_as_prefix() {
        let input = "ifconfig && thenext && elifprocess && elseware && fifile";
        let expected = vec![
            TokenKind::Word("ifconfig".to_string()),
            TokenKind::And,
            TokenKind::Word("thenext".to_string()),
            TokenKind::And,
            TokenKind::Word("elifprocess".to_string()),
            TokenKind::And,
            TokenKind::Word("elseware".to_string()),
            TokenKind::And,
            TokenKind::Word("fifile".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_words_with_glob_patterns() {
        let input = "ls *.txt file?.log [abc]*.tmp";
        let expected = vec![
            TokenKind::Word("ls".to_string()),
            TokenKind::Word("*.txt".to_string()),
            TokenKind::Word("file?.log".to_string()),
            TokenKind::Word("[abc]*.tmp".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_extglob_patterns() {
        // Test extended glob patterns
        let input = "ls ?(file|temp).txt *(a|b|c).log +(1|2|3).dat";
        let expected = vec![
            TokenKind::Word("ls".to_string()),
            TokenKind::Word("?(file|temp).txt".to_string()),
            TokenKind::Word("*(a|b|c).log".to_string()),
            TokenKind::Word("+(1|2|3).dat".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_mixed_keywords_and_words() {
        let input = "if if_var=42; then echo then_var=42; fi";
        let expected = vec![
            TokenKind::If,
            TokenKind::Word("if_var".to_string()),
            TokenKind::Assignment,
            TokenKind::Word("42".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("then_var".to_string()),
            TokenKind::Assignment,
            TokenKind::Word("42".to_string()),
            TokenKind::Semicolon,
            TokenKind::Fi,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_command_substitution_in_if() {
        let input = "if $(test -d /tmp); then echo directory exists; fi";
        let expected = vec![
            TokenKind::If,
            TokenKind::CmdSubst,
            TokenKind::Word("test".to_string()),
            TokenKind::Word("-d".to_string()),
            TokenKind::Word("/tmp".to_string()),
            TokenKind::RParen,
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("directory".to_string()),
            TokenKind::Word("exists".to_string()),
            TokenKind::Semicolon,
            TokenKind::Fi,
        ];
        test_tokens(input, expected);
    }
}
