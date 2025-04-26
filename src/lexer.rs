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
            },
            '\n' => {
                self.line += 1;
                self.column = 0;
                Token {
                    kind: TokenKind::Newline,
                    value: "\n".to_string(),
                    position: current_position,
                }
            },
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
            },
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
            },
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

        Token {
            kind: TokenKind::Word(word.clone()),
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
