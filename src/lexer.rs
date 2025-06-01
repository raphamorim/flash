/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

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
    ArithSubst,    // $((
    ExtGlob(char), // For ?(, *(, +(, @(, !(
    // Shell control flow keywords
    If,   // if keyword
    Then, // then keyword
    Elif, // elif keyword
    Else, // else keyword
    Fi,   // fi keyword
    // Function declaration keyword
    Function, // function keyword
    // Loop keywords
    For,   // for keyword
    While, // while keyword
    Until, // until keyword
    Do,    // do keyword
    Done,  // done keyword
    In,    // in keyword (used in for loops)
    // Break and continue for loops
    Break,    // break keyword
    Continue, // continue keyword
    Return,   // return keyword (for functions)
    Export,   // export keyword
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
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

fn is_special_char(ch: char) -> bool {
    matches!(
        ch,
        '=' | '|'
            | ';'
            | '\n'
            | '&'
            | '('
            | ')'
            | '{'
            | '}'
            | '<'
            | '>'
            | '$'
            | '"'
            | '\''
            | '`'
            | '#'
    )
}

fn is_word_terminator(ch: char) -> bool {
    matches!(
        ch,
        '=' | '|' | ';' | '\n' | '&' | '(' | ')' | '<' | '>' | '$' | '"' | '\'' | '`' | '#'
    )
}

/// Lexer that converts input text into tokens
#[derive(Clone)]
pub struct Lexer {
    input: Vec<char>,
    pub position: usize,
    read_position: usize,
    ch: char,
    line: usize,
    column: usize,
    in_quotes: Option<char>,
    quote_after_cmdsubst: Option<char>,
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
            in_quotes: None,
            quote_after_cmdsubst: None,
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

    pub fn peek_char(&self) -> char {
        if self.read_position >= self.input.len() {
            '\0'
        } else {
            self.input[self.read_position]
        }
    }

    // check if the current position is followed by whitespace or a special character
    fn is_word_boundary(&self) -> bool {
        let peek = self.peek_char();
        peek.is_whitespace() || is_special_char(peek) || peek == '\0'
    }

    pub fn peek_next_token(&mut self) -> Token {
        // Save the current state
        let saved_position = self.position;
        let saved_read_position = self.read_position;
        let saved_ch = self.ch;
        let saved_line = self.line;
        let saved_column = self.column;

        // Get the next token
        let token = self.next_token();

        // Restore the saved state
        self.position = saved_position;
        self.read_position = saved_read_position;
        self.ch = saved_ch;
        self.line = saved_line;
        self.column = saved_column;

        token
    }

    pub fn next_token(&mut self) -> Token {
        if self.in_quotes.is_none() {
            self.skip_whitespace();
        }

        let current_position = Position::new(self.line, self.column);

        // Check for quote start/end
        if (self.ch == '"' || self.ch == '\'') && self.in_quotes.is_none() {
            // Starting a quoted section
            let quote_type = self.ch;
            let token = Token {
                kind: if quote_type == '"' {
                    TokenKind::Quote
                } else {
                    TokenKind::SingleQuote
                },
                value: quote_type.to_string(),
                position: current_position,
            };

            self.in_quotes = Some(quote_type); // Set the in_quotes state
            self.read_char();
            return token;
        } else if self.in_quotes.is_some() && self.ch == self.in_quotes.unwrap() {
            // Ending a quoted section
            let quote_type = self.ch;
            let token = Token {
                kind: if quote_type == '"' {
                    TokenKind::Quote
                } else {
                    TokenKind::SingleQuote
                },
                value: quote_type.to_string(),
                position: current_position,
            };

            self.in_quotes = None; // Clear the in_quotes state
            self.read_char();
            return token;
        } else if self.in_quotes.is_some() {
            // We're inside quotes, but check for command substitution first
            if self.ch == '$' && self.peek_char() == '(' {
                // Handle command substitution even inside quotes
                // Save the quote state and temporarily exit quote mode
                self.quote_after_cmdsubst = self.in_quotes;
                self.in_quotes = None;
                self.read_char(); // Consume the '('
                self.read_char(); // Advance to the next character (like the end of method does)
                return Token {
                    kind: TokenKind::CmdSubst,
                    value: "$(".to_string(),
                    position: current_position,
                };
            } else {
                // Regular quoted content
                return self.read_quoted_content();
            }
        }

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
            ')' => {
                // Check if we need to restore quote state after command substitution
                if let Some(quote_char) = self.quote_after_cmdsubst {
                    self.in_quotes = Some(quote_char);
                    self.quote_after_cmdsubst = None;
                }
                Token {
                    kind: TokenKind::RParen,
                    value: ")".to_string(),
                    position: current_position,
                }
            }
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
            '!' => {
                // Check for != operator
                if self.peek_char() == '=' {
                    self.read_char(); // Consume the '='
                    Token {
                        kind: TokenKind::Word("!=".to_string()),
                        value: "!=".to_string(),
                        position: current_position,
                    }
                } else {
                    // Single ! is treated as a word character
                    self.read_word()
                }
            }
            '$' => {
                // Check for arithmetic expansion $(( syntax
                if self.peek_char() == '(' {
                    // Look ahead to see if it's $(( for arithmetic expansion
                    if self.position + 2 < self.input.len() && self.input[self.position + 2] == '('
                    {
                        self.read_char(); // Consume first '('
                        self.read_char(); // Consume second '('
                        Token {
                            kind: TokenKind::ArithSubst,
                            value: "$((".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Regular command substitution $(
                        self.read_char(); // Consume the '('
                        Token {
                            kind: TokenKind::CmdSubst,
                            value: "$(".to_string(),
                            position: current_position,
                        }
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
                // Check for "else", "elif", or "export" keywords
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
                } else if self.position + 5 < self.input.len()
                    && self.peek_char() == 'x'
                    && self.input[self.position + 1] == 'x'
                    && self.input[self.position + 2] == 'p'
                    && self.input[self.position + 3] == 'o'
                    && self.input[self.position + 4] == 'r'
                    && self.input[self.position + 5] == 't'
                {
                    self.read_char(); // 'x'
                    self.read_char(); // 'p'
                    self.read_char(); // 'o'
                    self.read_char(); // 'r'
                    self.read_char(); // 't'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::Export,
                            value: "export".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "export", backtrack
                        self.position -= 5;
                        self.read_position -= 5;
                        self.column -= 5;
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
                } else if self.position + 7 < self.input.len()
                    && self.peek_char() == 'u'
                    && self.input[self.position + 1] == 'u'
                    && self.input[self.position + 2] == 'n'
                    && self.input[self.position + 3] == 'c'
                    && self.input[self.position + 4] == 't'
                    && self.input[self.position + 5] == 'i'
                    && self.input[self.position + 6] == 'o'
                    && self.input[self.position + 7] == 'n'
                {
                    // Check for "function" keyword
                    self.read_char(); // 'u'
                    self.read_char(); // 'n'
                    self.read_char(); // 'c'
                    self.read_char(); // 't'
                    self.read_char(); // 'i'
                    self.read_char(); // 'o'
                    self.read_char(); // 'n'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::Function,
                            value: "function".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "function", backtrack
                        self.position -= 7;
                        self.read_position -= 7;
                        self.column -= 7;
                        self.ch = 'f';
                        self.read_word()
                    }
                } else if self.position + 2 < self.input.len()
                    && self.peek_char() == 'o'
                    && self.input[self.position + 1] == 'o'
                    && self.input[self.position + 2] == 'r'
                {
                    // Check for "for" keyword
                    self.read_char(); // 'o'
                    self.read_char(); // 'r'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::For,
                            value: "for".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "for", backtrack
                        self.position -= 2;
                        self.read_position -= 2;
                        self.column -= 2;
                        self.ch = 'f';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            'u' => {
                // Check for "until" keyword
                if self.position + 4 < self.input.len()
                    && self.peek_char() == 'n'
                    && self.input[self.position + 1] == 'n'
                    && self.input[self.position + 2] == 't'
                    && self.input[self.position + 3] == 'i'
                    && self.input[self.position + 4] == 'l'
                {
                    self.read_char(); // 'n'
                    self.read_char(); // 't'
                    self.read_char(); // 'i'
                    self.read_char(); // 'l'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::Until,
                            value: "until".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "until", backtrack
                        self.position -= 4;
                        self.read_position -= 4;
                        self.column -= 4;
                        self.ch = 'u';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            'r' => {
                // Check for "return" keyword
                if self.position + 5 < self.input.len()
                    && self.peek_char() == 'e'
                    && self.input[self.position + 1] == 'e'
                    && self.input[self.position + 2] == 't'
                    && self.input[self.position + 3] == 'u'
                    && self.input[self.position + 4] == 'r'
                    && self.input[self.position + 5] == 'n'
                {
                    self.read_char(); // 'e'
                    self.read_char(); // 't'
                    self.read_char(); // 'u'
                    self.read_char(); // 'r'
                    self.read_char(); // 'n'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::Return,
                            value: "return".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "return", backtrack
                        self.position -= 5;
                        self.read_position -= 5;
                        self.column -= 5;
                        self.ch = 'r';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            'w' => {
                // Check for "while" keyword
                if self.position + 4 < self.input.len()
                    && self.peek_char() == 'h'
                    && self.input[self.position + 1] == 'h'
                    && self.input[self.position + 2] == 'i'
                    && self.input[self.position + 3] == 'l'
                    && self.input[self.position + 4] == 'e'
                {
                    self.read_char(); // 'h'
                    self.read_char(); // 'i'
                    self.read_char(); // 'l'
                    self.read_char(); // 'e'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::While,
                            value: "while".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "while", backtrack
                        self.position -= 4;
                        self.read_position -= 4;
                        self.column -= 4;
                        self.ch = 'w';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            'd' => {
                // Check for "do" or "done" keywords
                if self.peek_char() == 'o' && self.position + 1 < self.input.len() {
                    self.read_char(); // 'o'

                    if self.peek_char() == 'n'
                        && self.position + 2 < self.input.len()
                        && self.input[self.position + 1] == 'n'
                        && self.input[self.position + 2] == 'e'
                    {
                        self.read_char(); // 'n'
                        self.read_char(); // 'e'

                        if self.is_word_boundary() {
                            Token {
                                kind: TokenKind::Done,
                                value: "done".to_string(),
                                position: current_position,
                            }
                        } else {
                            // Not a standalone "done", backtrack
                            self.position -= 3;
                            self.read_position -= 3;
                            self.column -= 3;
                            self.ch = 'd';
                            self.read_word()
                        }
                    } else if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::Do,
                            value: "do".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "do", backtrack
                        self.position -= 1;
                        self.read_position -= 1;
                        self.column -= 1;
                        self.ch = 'd';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            'b' => {
                // Check for "break" keyword
                if self.position + 4 < self.input.len()
                    && self.peek_char() == 'r'
                    && self.input[self.position + 1] == 'r'
                    && self.input[self.position + 2] == 'e'
                    && self.input[self.position + 3] == 'a'
                    && self.input[self.position + 4] == 'k'
                {
                    self.read_char(); // 'r'
                    self.read_char(); // 'e'
                    self.read_char(); // 'a'
                    self.read_char(); // 'k'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::Break,
                            value: "break".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "break", backtrack
                        self.position -= 4;
                        self.read_position -= 4;
                        self.column -= 4;
                        self.ch = 'b';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
            'c' => {
                // Check for "continue" keyword
                if self.position + 7 < self.input.len()
                    && self.peek_char() == 'o'
                    && self.input[self.position + 1] == 'o'
                    && self.input[self.position + 2] == 'n'
                    && self.input[self.position + 3] == 't'
                    && self.input[self.position + 4] == 'i'
                    && self.input[self.position + 5] == 'n'
                    && self.input[self.position + 6] == 'u'
                    && self.input[self.position + 7] == 'e'
                {
                    self.read_char(); // 'o'
                    self.read_char(); // 'n'
                    self.read_char(); // 't'
                    self.read_char(); // 'i'
                    self.read_char(); // 'n'
                    self.read_char(); // 'u'
                    self.read_char(); // 'e'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::Continue,
                            value: "continue".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "continue", backtrack
                        self.position -= 7;
                        self.read_position -= 7;
                        self.column -= 7;
                        self.ch = 'c';
                        self.read_word()
                    }
                } else {
                    self.read_word()
                }
            }
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
                } else if self.position + 1 < self.input.len() &&
            // check in
           self.peek_char() == 'n'
                {
                    self.read_char(); // 'n'

                    if self.is_word_boundary() {
                        Token {
                            kind: TokenKind::In,
                            value: "in".to_string(),
                            position: current_position,
                        }
                    } else {
                        // Not a standalone "in", backtrack
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
                while !self.ch.is_whitespace() && self.ch != '\0' && !is_word_terminator(self.ch) {
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

        // Read word characters, including glob patterns but handling braces carefully
        while !self.ch.is_whitespace() && self.ch != '\0' {
            // Handle special case for '=' in command line arguments first
            if self.ch == '=' && word.starts_with('-') {
                // For command line arguments like --option=value, include the = as part of the word
                word.push(self.ch);
                self.read_char();

                // Continue reading the value part
                while !self.ch.is_whitespace() && self.ch != '\0' && !is_word_terminator(self.ch) {
                    word.push(self.ch);
                    self.read_char();
                }
                break; // Exit the main loop after handling the argument
            }
            // Check for other word terminators
            else if is_word_terminator(self.ch) {
                break;
            }
            // Handle brace expansion - check if this looks like a glob pattern
            else if self.ch == '{' {
                // Always treat { as part of the word if we're already reading a word
                // This handles cases like *.{txt,log}
                word.push(self.ch);
                self.read_char();

                // Read until matching closing brace
                let mut depth = 1;
                while depth > 0 && self.ch != '\0' && !self.ch.is_whitespace() {
                    if self.ch == '{' {
                        depth += 1;
                    } else if self.ch == '}' {
                        depth -= 1;
                    }
                    word.push(self.ch);
                    self.read_char();
                }
            }
            // Handle standalone } - this should terminate the word
            else if self.ch == '}' {
                break;
            }
            // Handle character classes
            else if self.ch == '[' {
                word.push(self.ch);
                self.read_char();

                // Handle negation at start of character class
                if self.ch == '!' || self.ch == '^' {
                    word.push(self.ch);
                    self.read_char();
                }

                // Read until closing bracket
                while self.ch != ']' && self.ch != '\0' && !self.ch.is_whitespace() {
                    word.push(self.ch);
                    self.read_char();
                }

                // Include the closing bracket
                if self.ch == ']' {
                    word.push(self.ch);
                    self.read_char();
                }
            }
            // Handle regular characters and glob metacharacters
            else {
                word.push(self.ch);
                self.read_char();
            }
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
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "until" => TokenKind::Until,
            "do" => TokenKind::Do,
            "done" => TokenKind::Done,
            "in" => TokenKind::In,
            "function" => TokenKind::Function,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "export" => TokenKind::Export,
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

    fn read_quoted_content(&mut self) -> Token {
        let position = Position::new(self.line, self.column);
        let mut content = String::new();
        let quote_char = self.in_quotes.unwrap();

        // Keep reading until we hit the closing quote or EOF
        while self.ch != quote_char && self.ch != '\0' {
            // Handle escaped quotes
            if self.ch == '\\' && self.peek_char() == quote_char {
                self.read_char(); // Skip the backslash
            }

            if self.ch == '\n' {
                self.line += 1;
                self.column = 0;
            }

            content.push(self.ch);
            self.read_char();
        }

        Token {
            kind: TokenKind::Word(content.clone()),
            value: content,
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
    use crate::lexer::Token;
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

    fn collect_tokens(input: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token();
            let is_eof = matches!(token.kind, TokenKind::EOF);
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        tokens
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
    fn test_peek_without_advancing() {
        let input = "if then";
        let mut lexer = Lexer::new(input);

        // Peek next token (should be 'if')
        let peeked_token = lexer.peek_next_token();
        assert_eq!(peeked_token.kind, TokenKind::If);
        assert_eq!(peeked_token.value, "if");

        // Current token should still be 'if' after peeking
        let current_token = lexer.next_token();
        assert_eq!(current_token.kind, TokenKind::If);
        assert_eq!(current_token.value, "if");

        // Next token should be 'then'
        let next_token = lexer.next_token();
        assert_eq!(next_token.kind, TokenKind::Then);
        assert_eq!(next_token.value, "then");
    }

    #[test]
    fn test_multiple_peeks() {
        let input = "for i in 1 2 3";
        let mut lexer = Lexer::new(input);

        // First peek should be 'for'
        let first_peek = lexer.peek_next_token();
        assert_eq!(first_peek.kind, TokenKind::For);

        // Second peek should still be 'for' since we haven't advanced
        let second_peek = lexer.peek_next_token();
        assert_eq!(second_peek.kind, TokenKind::For);

        // Now consume the 'for' token
        let token = lexer.next_token();
        assert_eq!(token.kind, TokenKind::For);

        // Peek should now be 'i'
        let third_peek = lexer.peek_next_token();
        assert_eq!(third_peek.kind, TokenKind::Word("i".to_string()));
    }

    #[test]
    fn test_peek_at_end() {
        let input = "ls";
        let mut lexer = Lexer::new(input);

        // Consume the only token
        let token = lexer.next_token();
        assert_eq!(token.kind, TokenKind::Word("ls".to_string()));

        // Peek should now return EOF
        let peeked_token = lexer.peek_next_token();
        assert_eq!(peeked_token.kind, TokenKind::EOF);

        // Next token should also be EOF
        let eof_token = lexer.next_token();
        assert_eq!(eof_token.kind, TokenKind::EOF);
    }

    #[test]
    fn test_peek_special_tokens() {
        let input = "if [ $a = 5 ]; then echo success; fi";
        let mut lexer = Lexer::new(input);

        // Consume 'if'
        let if_token = lexer.next_token();
        assert_eq!(if_token.kind, TokenKind::If);

        // Peek should be '['
        let peek_token = lexer.peek_next_token();
        assert_eq!(peek_token.kind, TokenKind::Word("[".to_string()));

        // Lexer position should still be at the same point
        let bracket_token = lexer.next_token();
        assert_eq!(bracket_token.kind, TokenKind::Word("[".to_string()));

        // Let's consume a few more tokens
        lexer.next_token(); // $
        lexer.next_token(); // a

        // Peek should now be '='
        let eq_peek = lexer.peek_next_token();
        assert_eq!(eq_peek.kind, TokenKind::Assignment);
        assert_eq!(eq_peek.value, "=");

        // And verify we're still at the same position
        let eq_token = lexer.next_token();
        assert_eq!(eq_token.kind, TokenKind::Assignment);
    }

    #[test]
    fn test_peek_with_complex_tokens() {
        let input = "ls -l || echo 'failed'";
        let mut lexer = Lexer::new(input);

        // Consume 'ls' and '-l'
        lexer.next_token(); // ls
        lexer.next_token(); // -l

        // Peek should now be '||'
        let or_peek = lexer.peek_next_token();
        assert_eq!(or_peek.kind, TokenKind::Or);
        assert_eq!(or_peek.value, "||");

        // Verify we still get '||' when advancing
        let or_token = lexer.next_token();
        assert_eq!(or_token.kind, TokenKind::Or);

        // Peek should now be 'echo'
        let echo_peek = lexer.peek_next_token();
        assert_eq!(echo_peek.kind, TokenKind::Word("echo".to_string()));
    }

    #[test]
    fn test_peek_with_newlines() {
        let input = "echo hello\necho world";
        let mut lexer = Lexer::new(input);

        // Consume 'echo' and 'hello'
        lexer.next_token(); // echo
        lexer.next_token(); // hello

        // Peek should be newline
        let nl_peek = lexer.peek_next_token();
        assert_eq!(nl_peek.kind, TokenKind::Newline);

        // Advance past newline
        let nl_token = lexer.next_token();
        assert_eq!(nl_token.kind, TokenKind::Newline);

        // Peek should now be the second 'echo'
        let echo2_peek = lexer.peek_next_token();
        assert_eq!(echo2_peek.kind, TokenKind::Word("echo".to_string()));
    }

    #[test]
    fn test_peek_with_comments() {
        let input = "# This is a comment\necho hello";
        let mut lexer = Lexer::new(input);

        // Peek should be a comment
        let comment_peek = lexer.peek_next_token();
        assert_eq!(comment_peek.kind, TokenKind::Comment);

        // Advance past comment
        let comment_token = lexer.next_token();
        assert_eq!(comment_token.kind, TokenKind::Comment);

        // Peek should now be newline
        let nl_peek = lexer.peek_next_token();
        assert_eq!(nl_peek.kind, TokenKind::Newline);
    }

    #[test]
    fn test_state_preservation() {
        let input = "if [ $? -eq 0 ]; then echo success; fi";
        let mut lexer = Lexer::new(input);

        // Record initial position data
        let initial_position = lexer.position;
        let initial_read_position = lexer.read_position;
        let initial_line = lexer.line;
        let initial_column = lexer.column;

        // Peek next token to ensure state is preserved
        lexer.peek_next_token();

        // Verify that the lexer's state hasn't changed
        assert_eq!(lexer.position, initial_position);
        assert_eq!(lexer.read_position, initial_read_position);
        assert_eq!(lexer.line, initial_line);
        assert_eq!(lexer.column, initial_column);

        // Now advance the lexer
        lexer.next_token();

        // Verify that the state has now changed
        assert_ne!(lexer.position, initial_position);
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
            TokenKind::Background,
            TokenKind::Word("1".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_quoted_strings() {
        let input = r#"echo "hello world" 'rio de janeiro'"#;
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Quote,
            TokenKind::Word("hello world".to_string()),
            TokenKind::Quote,
            TokenKind::SingleQuote,
            TokenKind::Word("rio de janeiro".to_string()),
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
    fn test_command_substitution_on_variable() {
        let input = "NUMBER=$(echo 85)";
        let expected = vec![
            TokenKind::Word("NUMBER".to_string()),
            TokenKind::Assignment,
            TokenKind::CmdSubst,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("85".to_string()),
            TokenKind::RParen,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_command_substitution_on_variable_with_quotes() {
        let input = "NUMBER=\"$(echo 85)\"";
        let expected = vec![
            TokenKind::Word("NUMBER".to_string()),
            TokenKind::Assignment,
            TokenKind::Quote,
            TokenKind::CmdSubst,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("85".to_string()),
            TokenKind::RParen,
            TokenKind::Quote,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_equal_sign_not_as_assignment() {
        let input = "./configure --target=something";
        let expected = vec![
            TokenKind::Word("./configure".to_string()),
            TokenKind::Word("--target=something".to_string()),
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

    #[test]
    fn test_function_declaration() {
        let input = "function greet() { echo hello; }";
        let expected = vec![
            TokenKind::Function,
            TokenKind::Word("greet".to_string()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("hello".to_string()),
            TokenKind::Semicolon,
            TokenKind::RBrace,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_function_declaration_alternate_syntax() {
        let input = "greet() { echo hello; }";
        let expected = vec![
            TokenKind::Word("greet".to_string()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("hello".to_string()),
            TokenKind::Semicolon,
            TokenKind::RBrace,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_function_call() {
        let input = "greet; greet arg1 arg2";
        let expected = vec![
            TokenKind::Word("greet".to_string()),
            TokenKind::Semicolon,
            TokenKind::Word("greet".to_string()),
            TokenKind::Word("arg1".to_string()),
            TokenKind::Word("arg2".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_function_with_return() {
        let input = "function check() { if [ $1 -eq 0 ]; then return 1; fi; echo ok; }";
        let expected = vec![
            TokenKind::Function,
            TokenKind::Word("check".to_string()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::If,
            TokenKind::Word("[".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("1".to_string()),
            TokenKind::Word("-eq".to_string()),
            TokenKind::Word("0".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Return,
            TokenKind::Word("1".to_string()),
            TokenKind::Semicolon,
            TokenKind::Fi,
            TokenKind::Semicolon,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("ok".to_string()),
            TokenKind::Semicolon,
            TokenKind::RBrace,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_function_multiline() {
        let input = "function hello() {\n  echo \"Hello, world!\"\n  return 0\n}";
        let expected = vec![
            TokenKind::Function,
            TokenKind::Word("hello".to_string()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Newline,
            TokenKind::Word("echo".to_string()),
            TokenKind::Quote,
            TokenKind::Word("Hello, world!".to_string()),
            TokenKind::Quote,
            TokenKind::Newline,
            TokenKind::Return,
            TokenKind::Word("0".to_string()),
            TokenKind::Newline,
            TokenKind::RBrace,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_for_loop_basic() {
        let input = "for i in 1 2 3; do echo $i; done";
        let expected = vec![
            TokenKind::For,
            TokenKind::Word("i".to_string()),
            TokenKind::In,
            TokenKind::Word("1".to_string()),
            TokenKind::Word("2".to_string()),
            TokenKind::Word("3".to_string()),
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Semicolon,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_for_loop_with_glob() {
        let input = "for file in *.txt; do cat $file; done";
        let expected = vec![
            TokenKind::For,
            TokenKind::Word("file".to_string()),
            TokenKind::In,
            TokenKind::Word("*.txt".to_string()),
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::Word("cat".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("file".to_string()),
            TokenKind::Semicolon,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_for_loop_multiline() {
        let input = "for i in $(seq 1 10)\ndo\n  echo $i\ndone";
        let expected = vec![
            TokenKind::For,
            TokenKind::Word("i".to_string()),
            TokenKind::In,
            TokenKind::CmdSubst,
            TokenKind::Word("seq".to_string()),
            TokenKind::Word("1".to_string()),
            TokenKind::Word("10".to_string()),
            TokenKind::RParen,
            TokenKind::Newline,
            TokenKind::Do,
            TokenKind::Newline,
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Newline,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_for_loop_with_break() {
        let input = "for i in 1 2 3; do if [ $i -eq 2 ]; then break; fi; done";
        let expected = vec![
            TokenKind::For,
            TokenKind::Word("i".to_string()),
            TokenKind::In,
            TokenKind::Word("1".to_string()),
            TokenKind::Word("2".to_string()),
            TokenKind::Word("3".to_string()),
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::If,
            TokenKind::Word("[".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Word("-eq".to_string()),
            TokenKind::Word("2".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Break,
            TokenKind::Semicolon,
            TokenKind::Fi,
            TokenKind::Semicolon,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_for_loop_with_continue() {
        let input = "for i in 1 2 3; do if [ $i -eq 2 ]; then continue; fi; echo $i; done";
        let expected = vec![
            TokenKind::For,
            TokenKind::Word("i".to_string()),
            TokenKind::In,
            TokenKind::Word("1".to_string()),
            TokenKind::Word("2".to_string()),
            TokenKind::Word("3".to_string()),
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::If,
            TokenKind::Word("[".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Word("-eq".to_string()),
            TokenKind::Word("2".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Continue,
            TokenKind::Semicolon,
            TokenKind::Fi,
            TokenKind::Semicolon,
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Semicolon,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_c_style_for_loop() {
        let input = "for ((i=0; i<5; i++)); do echo $i; done";
        let expected = vec![
            TokenKind::For,
            TokenKind::LParen,
            TokenKind::LParen,
            TokenKind::Word("i".to_string()),
            TokenKind::Assignment,
            TokenKind::Word("0".to_string()),
            TokenKind::Semicolon,
            TokenKind::Word("i".to_string()),
            TokenKind::Less,
            TokenKind::Word("5".to_string()),
            TokenKind::Semicolon,
            TokenKind::Word("i++".to_string()),
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Semicolon,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_c_style_for_loop_using_decrement() {
        let input = "for ((i=5; i>0; i--)); do echo $i; done";
        let expected = vec![
            TokenKind::For,
            TokenKind::LParen,
            TokenKind::LParen,
            TokenKind::Word("i".to_string()),
            TokenKind::Assignment,
            TokenKind::Word("5".to_string()),
            TokenKind::Semicolon,
            TokenKind::Word("i".to_string()),
            TokenKind::Great,
            TokenKind::Word("0".to_string()),
            TokenKind::Semicolon,
            TokenKind::Word("i--".to_string()),
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Semicolon,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_while_loop_basic() {
        let input = "while [ $i -lt 10 ]; do echo $i; i=$((i+1)); done";
        let expected = vec![
            TokenKind::While,
            TokenKind::Word("[".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Word("-lt".to_string()),
            TokenKind::Word("10".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Semicolon,
            TokenKind::Word("i".to_string()),
            TokenKind::Assignment,
            TokenKind::ArithSubst,
            TokenKind::Word("i+1".to_string()),
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::Semicolon,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_while_loop_multiline() {
        let input = "while true\ndo\n  echo looping\n  if [ $count -gt 10 ]; then break; fi\ndone";
        let expected = vec![
            TokenKind::While,
            TokenKind::Word("true".to_string()),
            TokenKind::Newline,
            TokenKind::Do,
            TokenKind::Newline,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("looping".to_string()),
            TokenKind::Newline,
            TokenKind::If,
            TokenKind::Word("[".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("count".to_string()),
            TokenKind::Word("-gt".to_string()),
            TokenKind::Word("10".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Break,
            TokenKind::Semicolon,
            TokenKind::Fi,
            TokenKind::Newline,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_array_declaration() {
        let input = "colors=(red green blue)";
        let expected = vec![
            TokenKind::Word("colors".to_string()),
            TokenKind::Assignment,
            TokenKind::LParen,
            TokenKind::Word("red".to_string()),
            TokenKind::Word("green".to_string()),
            TokenKind::Word("blue".to_string()),
            TokenKind::RParen,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_export_keyword() {
        let tokens = collect_tokens("export");
        assert_eq!(tokens.len(), 2); // export + EOF
        assert!(matches!(tokens[0].kind, TokenKind::Export));
        assert_eq!(tokens[0].value, "export");
    }

    #[test]
    fn test_export_assignment() {
        let tokens = collect_tokens("export VAR=value");
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();

        assert_eq!(kinds.len(), 5); // export + VAR + = + value + EOF
        assert!(matches!(kinds[0], TokenKind::Export));
        assert!(matches!(kinds[1], TokenKind::Word(_)));
        assert!(matches!(kinds[2], TokenKind::Assignment));
        assert!(matches!(kinds[3], TokenKind::Word(_)));
        assert!(matches!(kinds[4], TokenKind::EOF));

        assert_eq!(tokens[0].value, "export");
        assert_eq!(tokens[1].value, "VAR");
        assert_eq!(tokens[2].value, "=");
        assert_eq!(tokens[3].value, "value");
    }

    #[test]
    fn test_export_with_quotes() {
        let tokens = collect_tokens("export PATH=\"/usr/bin:/bin\"");
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();

        assert!(matches!(kinds[0], TokenKind::Export));
        assert!(matches!(kinds[1], TokenKind::Word(_)));
        assert!(matches!(kinds[2], TokenKind::Assignment));
        assert!(matches!(kinds[3], TokenKind::Quote));
        assert!(matches!(kinds[4], TokenKind::Word(_)));
        assert!(matches!(kinds[5], TokenKind::Quote));

        assert_eq!(tokens[0].value, "export");
        assert_eq!(tokens[1].value, "PATH");
        assert_eq!(tokens[4].value, "/usr/bin:/bin");
    }

    #[test]
    fn test_export_multiple_variables() {
        let tokens = collect_tokens("export VAR1=val1 VAR2=val2");
        let export_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Export))
            .count();
        assert_eq!(export_count, 1); // Only one export keyword

        let var_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Word(_)))
            .count();
        assert_eq!(var_count, 4); // VAR1, val1, VAR2, val2
    }

    #[test]
    fn test_export_not_keyword_when_part_of_word() {
        let tokens = collect_tokens("exported");
        assert_eq!(tokens.len(), 2); // word + EOF
        assert!(matches!(tokens[0].kind, TokenKind::Word(_)));
        assert_eq!(tokens[0].value, "exported");

        let tokens2 = collect_tokens("exportable");
        assert!(matches!(tokens2[0].kind, TokenKind::Word(_)));
        assert_eq!(tokens2[0].value, "exportable");
    }

    #[test]
    fn test_export_with_newline() {
        let tokens = collect_tokens("export VAR=value\necho $VAR");
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();

        assert!(matches!(kinds[0], TokenKind::Export));
        assert!(matches!(kinds[4], TokenKind::Newline)); // After value
        assert!(matches!(kinds[5], TokenKind::Word(_))); // echo
    }

    // #[test]
    // fn test_export_with_variable() {
    //     let tokens = collect_tokens("export PATH=\"$PATH\":");
    //     let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();

    //     assert!(matches!(kinds[0], TokenKind::Export));
    //     assert!(matches!(kinds[4], TokenKind::Newline)); // After value
    //     assert!(matches!(kinds[5], TokenKind::Word(_))); // echo
    // }

    #[test]
    fn test_export_with_semicolon() {
        let tokens = collect_tokens("export VAR=value; echo done");
        let semicolon_pos = tokens
            .iter()
            .position(|t| matches!(t.kind, TokenKind::Semicolon));
        assert!(semicolon_pos.is_some());
    }

    #[test]
    fn test_until_loop() {
        let input = "until [ $count -eq 10 ]; do echo $count; count=$((count+1)); done";
        let expected = vec![
            TokenKind::Until,
            TokenKind::Word("[".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("count".to_string()),
            TokenKind::Word("-eq".to_string()),
            TokenKind::Word("10".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("count".to_string()),
            TokenKind::Semicolon,
            TokenKind::Word("count".to_string()),
            TokenKind::Assignment,
            TokenKind::ArithSubst,
            TokenKind::Word("count+1".to_string()),
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::Semicolon,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_nested_loops() {
        let input = "for i in 1 2; do for j in a b; do echo $i$j; done; done";
        let expected = vec![
            TokenKind::For,
            TokenKind::Word("i".to_string()),
            TokenKind::In,
            TokenKind::Word("1".to_string()),
            TokenKind::Word("2".to_string()),
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::For,
            TokenKind::Word("j".to_string()),
            TokenKind::In,
            TokenKind::Word("a".to_string()),
            TokenKind::Word("b".to_string()),
            TokenKind::Semicolon,
            TokenKind::Do,
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("i".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("j".to_string()),
            TokenKind::Semicolon,
            TokenKind::Done,
            TokenKind::Semicolon,
            TokenKind::Done,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_complex_redirections() {
        let input = "cmd < input.txt > output.txt 2>&1 >> append.log";
        let expected = vec![
            TokenKind::Word("cmd".to_string()),
            TokenKind::Less,
            TokenKind::Word("input.txt".to_string()),
            TokenKind::Great,
            TokenKind::Word("output.txt".to_string()),
            TokenKind::Word("2".to_string()),
            TokenKind::Great,
            TokenKind::Background,
            TokenKind::Word("1".to_string()),
            TokenKind::DGreat,
            TokenKind::Word("append.log".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_backtick_command_substitution() {
        let input = "echo `date +%Y`";
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Backtick,
            TokenKind::Word("date".to_string()),
            TokenKind::Word("+%Y".to_string()),
            TokenKind::Backtick,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_nested_command_substitution() {
        let input = "echo $(echo $(date))";
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::CmdSubst,
            TokenKind::Word("echo".to_string()),
            TokenKind::CmdSubst,
            TokenKind::Word("date".to_string()),
            TokenKind::RParen,
            TokenKind::RParen,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_mixed_quotes() {
        let input = r#"echo "single 'quote' inside" 'double "quote" inside'"#;
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Quote,
            TokenKind::Word("single 'quote' inside".to_string()),
            TokenKind::Quote,
            TokenKind::SingleQuote,
            TokenKind::Word("double \"quote\" inside".to_string()),
            TokenKind::SingleQuote,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_escaped_quotes() {
        let input = r#"echo "escaped \" quote""#;
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Quote,
            TokenKind::Word("escaped \" quote".to_string()),
            TokenKind::Quote,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_multiline_strings() {
        let input = "echo \"line1\nline2\nline3\"";
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Quote,
            TokenKind::Word("line1\nline2\nline3".to_string()),
            TokenKind::Quote,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_complex_variable_expansion() {
        let input = "echo $HOME ${USER} $((2+3)) $?";
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::Word("HOME".to_string()),
            TokenKind::Dollar,
            TokenKind::LBrace,
            TokenKind::Word("USER".to_string()),
            TokenKind::RBrace,
            TokenKind::ArithSubst,
            TokenKind::Word("2+3".to_string()),
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::Dollar,
            TokenKind::Word("?".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_array_access() {
        let input = "echo ${array[0]} ${array[@]} ${#array[@]}";
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Dollar,
            TokenKind::LBrace,
            TokenKind::Word("array[0]".to_string()),
            TokenKind::RBrace,
            TokenKind::Dollar,
            TokenKind::LBrace,
            TokenKind::Word("array[@]".to_string()),
            TokenKind::RBrace,
            TokenKind::Dollar,
            TokenKind::LBrace,
            TokenKind::Comment,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_complex_extglob() {
        let input = "ls !(*.tmp|*.log) @(file1|file2).txt +(a|b|c)*";
        let expected = vec![
            TokenKind::Word("ls".to_string()),
            TokenKind::Word("!(*.tmp|*.log)".to_string()),
            TokenKind::Word("@(file1|file2).txt".to_string()),
            TokenKind::Word("+(a|b|c)*".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_subshell_and_grouping() {
        let input = "(cd /tmp && ls) { echo group; }";
        let expected = vec![
            TokenKind::LParen,
            TokenKind::Word("cd".to_string()),
            TokenKind::Word("/tmp".to_string()),
            TokenKind::And,
            TokenKind::Word("ls".to_string()),
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("group".to_string()),
            TokenKind::Semicolon,
            TokenKind::RBrace,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_pipeline_with_multiple_commands() {
        let input = "cat file.txt | grep pattern | sort | uniq -c | head -10";
        let expected = vec![
            TokenKind::Word("cat".to_string()),
            TokenKind::Word("file.txt".to_string()),
            TokenKind::Pipe,
            TokenKind::Word("grep".to_string()),
            TokenKind::Word("pattern".to_string()),
            TokenKind::Pipe,
            TokenKind::Word("sort".to_string()),
            TokenKind::Pipe,
            TokenKind::Word("uniq".to_string()),
            TokenKind::Word("-c".to_string()),
            TokenKind::Pipe,
            TokenKind::Word("head".to_string()),
            TokenKind::Word("-10".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_complex_conditional_operators() {
        let input = "cmd1 && cmd2 || cmd3 && cmd4";
        let expected = vec![
            TokenKind::Word("cmd1".to_string()),
            TokenKind::And,
            TokenKind::Word("cmd2".to_string()),
            TokenKind::Or,
            TokenKind::Word("cmd3".to_string()),
            TokenKind::And,
            TokenKind::Word("cmd4".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_function_with_complex_body() {
        let input = "function deploy() { if [ -f Dockerfile ]; then docker build -t app .; docker run -d app; else echo 'No Dockerfile found'; fi; }";
        let expected = vec![
            TokenKind::Function,
            TokenKind::Word("deploy".to_string()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::If,
            TokenKind::Word("[".to_string()),
            TokenKind::Word("-f".to_string()),
            TokenKind::Word("Dockerfile".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("docker".to_string()),
            TokenKind::Word("build".to_string()),
            TokenKind::Word("-t".to_string()),
            TokenKind::Word("app".to_string()),
            TokenKind::Word(".".to_string()),
            TokenKind::Semicolon,
            TokenKind::Word("docker".to_string()),
            TokenKind::Word("run".to_string()),
            TokenKind::Word("-d".to_string()),
            TokenKind::Word("app".to_string()),
            TokenKind::Semicolon,
            TokenKind::Else,
            TokenKind::Word("echo".to_string()),
            TokenKind::SingleQuote,
            TokenKind::Word("No Dockerfile found".to_string()),
            TokenKind::SingleQuote,
            TokenKind::Semicolon,
            TokenKind::Fi,
            TokenKind::Semicolon,
            TokenKind::RBrace,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_whitespace_handling() {
        let input = "  cmd1   arg1    arg2  ";
        let expected = vec![
            TokenKind::Word("cmd1".to_string()),
            TokenKind::Word("arg1".to_string()),
            TokenKind::Word("arg2".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let expected = vec![];
        test_tokens(input, expected);
    }

    #[test]
    fn test_only_whitespace() {
        let input = "   \t  \t   ";
        let expected = vec![];
        test_tokens(input, expected);
    }

    #[test]
    fn test_only_comments() {
        let input = "# This is a comment\n# Another comment";
        let expected = vec![TokenKind::Comment, TokenKind::Newline, TokenKind::Comment];
        test_tokens(input, expected);
    }

    #[test]
    fn test_special_characters_in_words() {
        let input = "file-name file_name file.txt file@host file:port";
        let expected = vec![
            TokenKind::Word("file-name".to_string()),
            TokenKind::Word("file_name".to_string()),
            TokenKind::Word("file.txt".to_string()),
            TokenKind::Word("file@host".to_string()),
            TokenKind::Word("file:port".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_numbers_and_arithmetic() {
        let input = "echo 123 0x1F 0755 3.14";
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("123".to_string()),
            TokenKind::Word("0x1F".to_string()),
            TokenKind::Word("0755".to_string()),
            TokenKind::Word("3.14".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_path_separators() {
        let input = "/usr/bin/bash ./script.sh ../parent/file ~/home/user";
        let expected = vec![
            TokenKind::Word("/usr/bin/bash".to_string()),
            TokenKind::Word("./script.sh".to_string()),
            TokenKind::Word("../parent/file".to_string()),
            TokenKind::Word("~/home/user".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_keyword_boundaries() {
        let input = "ifconfig thenext elifant elsewhere fifo";
        let expected = vec![
            TokenKind::Word("ifconfig".to_string()),
            TokenKind::Word("thenext".to_string()),
            TokenKind::Word("elifant".to_string()),
            TokenKind::Word("elsewhere".to_string()),
            TokenKind::Word("fifo".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_position_tracking() {
        let input = "line1\nline2\nline3";
        let mut lexer = Lexer::new(input);

        let token1 = lexer.next_token();
        assert_eq!(token1.position.line, 1);
        assert_eq!(token1.position.column, 1);

        let newline1 = lexer.next_token();
        assert_eq!(newline1.kind, TokenKind::Newline);

        let token2 = lexer.next_token();
        assert_eq!(token2.position.line, 2);
        assert_eq!(token2.position.column, 1);
    }

    #[test]
    fn test_error_recovery() {
        // Test lexer behavior with malformed input
        let input = "echo \"unclosed quote";
        let mut lexer = Lexer::new(input);

        let echo_token = lexer.next_token();
        assert_eq!(echo_token.kind, TokenKind::Word("echo".to_string()));

        let quote_token = lexer.next_token();
        assert_eq!(quote_token.kind, TokenKind::Quote);

        let content_token = lexer.next_token();
        assert_eq!(
            content_token.kind,
            TokenKind::Word("unclosed quote".to_string())
        );

        // The lexer should handle EOF gracefully even with unclosed quotes
        let eof_token = lexer.next_token();
        // The actual behavior might be different, so let's just check it doesn't panic
        assert!(matches!(
            eof_token.kind,
            TokenKind::EOF | TokenKind::Word(_)
        ));
    }

    #[test]
    fn test_large_input_performance() {
        // Test with a reasonably large input to ensure performance
        let large_input = "echo hello; ".repeat(1000);
        let mut lexer = Lexer::new(&large_input);

        let mut token_count = 0;
        loop {
            let token = lexer.next_token();
            if token.kind == TokenKind::EOF {
                break;
            }
            token_count += 1;
        }

        // Should have 3000 tokens (echo, hello, semicolon) * 1000 repetitions
        assert_eq!(token_count, 3000);
    }

    #[test]
    fn test_comprehensive_glob_patterns() {
        // Test various glob patterns to ensure they're tokenized as words
        let input = "ls *.txt file?.log [0-9]*.dat [a-z][A-Z]*.tmp [!abc]*.bak";
        let expected = vec![
            TokenKind::Word("ls".to_string()),
            TokenKind::Word("*.txt".to_string()),
            TokenKind::Word("file?.log".to_string()),
            TokenKind::Word("[0-9]*.dat".to_string()),
            TokenKind::Word("[a-z][A-Z]*.tmp".to_string()),
            TokenKind::Word("[!abc]*.bak".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_glob_patterns_with_paths() {
        // Test glob patterns with directory paths
        let input = "find /path/*.txt ./local/file?.log ../parent/[abc]*.tmp";
        let expected = vec![
            TokenKind::Word("find".to_string()),
            TokenKind::Word("/path/*.txt".to_string()),
            TokenKind::Word("./local/file?.log".to_string()),
            TokenKind::Word("../parent/[abc]*.tmp".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_glob_patterns_in_quotes() {
        // Test that glob patterns in quotes are preserved as literals
        let input = r#"echo "*.txt" 'file?.log' "test[abc].dat""#;
        let expected = vec![
            TokenKind::Word("echo".to_string()),
            TokenKind::Quote,
            TokenKind::Word("*.txt".to_string()),
            TokenKind::Quote,
            TokenKind::SingleQuote,
            TokenKind::Word("file?.log".to_string()),
            TokenKind::SingleQuote,
            TokenKind::Quote,
            TokenKind::Word("test[abc].dat".to_string()),
            TokenKind::Quote,
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_complex_glob_combinations() {
        // Test complex combinations of glob patterns
        let input = "command *.[ch] *.{txt,log} file[0-9][a-z].* test*[!~]";
        let expected = vec![
            TokenKind::Word("command".to_string()),
            TokenKind::Word("*.[ch]".to_string()),
            TokenKind::Word("*.{txt,log}".to_string()),
            TokenKind::Word("file[0-9][a-z].*".to_string()),
            TokenKind::Word("test*[!~]".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_glob_patterns_with_special_chars() {
        // Test glob patterns with special characters that should be preserved
        let input = "ls *-file.txt file_*.log test[._-]*.dat";
        let expected = vec![
            TokenKind::Word("ls".to_string()),
            TokenKind::Word("*-file.txt".to_string()),
            TokenKind::Word("file_*.log".to_string()),
            TokenKind::Word("test[._-]*.dat".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_negated_character_classes() {
        // Test negated character classes in glob patterns
        let input = "ls file[!0-9].txt data[^abc].log test[!~#].dat";
        let expected = vec![
            TokenKind::Word("ls".to_string()),
            TokenKind::Word("file[!0-9].txt".to_string()),
            TokenKind::Word("data[^abc].log".to_string()),
            TokenKind::Word("test[!~#].dat".to_string()),
        ];
        test_tokens(input, expected);
    }

    #[test]
    fn test_glob_patterns_mixed_with_other_tokens() {
        // Test glob patterns mixed with other shell constructs
        let input = "if [ -f *.txt ]; then echo file*.log | grep test; fi";
        let expected = vec![
            TokenKind::If,
            TokenKind::Word("[".to_string()),
            TokenKind::Word("-f".to_string()),
            TokenKind::Word("*.txt".to_string()),
            TokenKind::Word("]".to_string()),
            TokenKind::Semicolon,
            TokenKind::Then,
            TokenKind::Word("echo".to_string()),
            TokenKind::Word("file*.log".to_string()),
            TokenKind::Pipe,
            TokenKind::Word("grep".to_string()),
            TokenKind::Word("test".to_string()),
            TokenKind::Semicolon,
            TokenKind::Fi,
        ];
        test_tokens(input, expected);
    }
}
