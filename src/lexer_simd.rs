/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

//! SIMD-optimized lexer extensions.
//!
//! This module provides SIMD-accelerated versions of lexer operations
//! for improved performance on large shell scripts.

use crate::lexer::{Lexer, Position, Token, TokenKind};
use crate::simd::{find_newline, find_quotes, find_special_chars, find_whitespace};

impl Lexer {
    /// SIMD-optimized version of skip_whitespace.
    ///
    /// Uses vectorized operations to quickly find the next non-whitespace character.
    pub fn skip_whitespace_simd(&mut self) {
        let input = self.get_input();
        if input.is_empty() || self.position >= input.len() {
            return;
        }

        // Convert current position to byte slice for SIMD operations
        let input_bytes: Vec<u8> = input[self.position..].iter().map(|&c| c as u8).collect();

        if input_bytes.is_empty() {
            return;
        }

        let whitespace_pos = find_whitespace(&input_bytes, 0);

        if whitespace_pos == 0 {
            // We're at whitespace, skip it
            let mut chars_skipped = 0;
            for (i, &byte) in input_bytes.iter().enumerate() {
                if !matches!(byte, b' ' | b'\t' | b'\r') {
                    chars_skipped = i;
                    break;
                }
                if byte == b'\n' {
                    self.increment_line();
                    self.set_column(0);
                } else {
                    self.advance_column(1);
                }
            }

            self.advance_position(chars_skipped);
            let input = self.get_input();
            if self.position < input.len() {
                self.set_ch(input[self.position]);
            } else {
                self.set_ch('\0');
            }
        }
    }

    /// SIMD-optimized word reading.
    ///
    /// Uses vectorized operations to quickly find word boundaries.
    pub fn read_word_simd(&mut self) -> Token {
        let position = Position::new(self.get_line(), self.get_column());
        let mut word = String::new();

        let input = self.get_input();
        if self.position >= input.len() {
            return Token {
                kind: TokenKind::EOF,
                value: String::new(),
                position,
            };
        }

        // Convert remaining input to bytes for SIMD processing
        let input_bytes: Vec<u8> = input[self.position..].iter().map(|&c| c as u8).collect();

        // Find the next special character or whitespace
        let special_pos = find_special_chars(&input_bytes, 0);
        let whitespace_pos = find_whitespace(&input_bytes, 0);

        // Take the minimum of both positions
        let end_pos = special_pos.min(whitespace_pos);

        // Extract the word
        for i in 0..end_pos {
            if self.position + i < input.len() {
                word.push(input[self.position + i]);
            }
        }

        // Advance position
        self.advance_position(end_pos);
        self.advance_column(end_pos);

        let input = self.get_input();
        if self.position < input.len() {
            self.set_ch(input[self.position]);
        } else {
            self.set_ch('\0');
        }

        // Check if it's a keyword
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

    /// SIMD-optimized comment reading.
    ///
    /// Uses vectorized operations to quickly find the end of a comment.
    pub fn read_comment_simd(&mut self) -> Token {
        let position = Position::new(self.get_line(), self.get_column());
        let mut comment = String::from("#");

        self.read_char(); // Skip the '#'

        let input = self.get_input();
        if self.position >= input.len() {
            return Token {
                kind: TokenKind::Comment,
                value: comment,
                position,
            };
        }

        // Convert remaining input to bytes for SIMD processing
        let input_bytes: Vec<u8> = input[self.position..].iter().map(|&c| c as u8).collect();

        // Find the next newline
        let newline_pos = find_newline(&input_bytes, 0);

        // Extract the comment content
        for i in 0..newline_pos {
            if self.position + i < input.len() {
                comment.push(input[self.position + i]);
            }
        }

        // Advance position to just before the newline
        self.advance_position(newline_pos);
        self.advance_column(newline_pos);

        let input = self.get_input();
        if self.position < input.len() {
            self.set_ch(input[self.position]);
        } else {
            self.set_ch('\0');
        }

        Token {
            kind: TokenKind::Comment,
            value: comment,
            position,
        }
    }

    /// SIMD-optimized quoted string reading.
    ///
    /// Uses vectorized operations to quickly find the closing quote.
    pub fn read_quoted_content_simd(&mut self) -> Token {
        let position = Position::new(self.get_line(), self.get_column());
        let mut content = String::new();
        let quote_char = self.get_ch();

        self.read_char(); // Skip opening quote

        let input = self.get_input();
        if self.position >= input.len() {
            return Token {
                kind: if quote_char == '"' {
                    TokenKind::Quote
                } else {
                    TokenKind::SingleQuote
                },
                value: content,
                position,
            };
        }

        // For quoted strings, we need to be careful about escape sequences
        // So we'll use a hybrid approach: SIMD to find potential end quotes,
        // then validate them manually
        let input_bytes: Vec<u8> = input[self.position..].iter().map(|&c| c as u8).collect();

        let mut search_offset = 0;
        loop {
            let quote_pos = find_quotes(&input_bytes, search_offset);

            if quote_pos >= input_bytes.len() {
                // No closing quote found, read to end
                for i in search_offset..input_bytes.len() {
                    if self.position + i < input.len() {
                        content.push(input[self.position + i]);
                    }
                }
                self.advance_position(input_bytes.len() - search_offset);
                self.advance_column(input_bytes.len() - search_offset);
                break;
            }

            // Check if this quote matches our opening quote
            if input_bytes[quote_pos] == quote_char as u8 {
                // Check if it's escaped
                let mut escaped = false;
                if quote_pos > 0 && input_bytes[quote_pos - 1] == b'\\' {
                    // Count consecutive backslashes
                    let mut backslash_count = 0;
                    let mut check_pos = quote_pos - 1;
                    while check_pos < input_bytes.len()
                        && input_bytes[check_pos] == b'\\'
                        && check_pos > 0
                    {
                        backslash_count += 1;
                        check_pos -= 1;
                    }
                    escaped = backslash_count % 2 == 1;
                }

                if !escaped {
                    // Found unescaped closing quote
                    for i in search_offset..quote_pos {
                        if self.position + i < input.len() {
                            content.push(input[self.position + i]);
                        }
                    }
                    self.advance_position(quote_pos + 1); // +1 to skip the closing quote
                    self.advance_column(quote_pos + 1);
                    break;
                }
            }

            // Continue searching after this quote
            search_offset = quote_pos + 1;
        }

        let input = self.get_input();
        if self.position < input.len() {
            self.set_ch(input[self.position]);
        } else {
            self.set_ch('\0');
        }

        Token {
            kind: TokenKind::Word(content.clone()), // Use Word instead of StringLiteral
            value: content,
            position,
        }
    }

    /// SIMD-optimized next token method.
    ///
    /// Uses vectorized operations where possible to accelerate tokenization.
    pub fn next_token_simd(&mut self) -> Token {
        // Skip whitespace using SIMD
        self.skip_whitespace_simd();

        if self.get_ch() == '\0' {
            return Token {
                kind: TokenKind::EOF,
                value: String::new(),
                position: Position::new(self.get_line(), self.get_column()),
            };
        }

        // Handle common cases with SIMD optimizations
        match self.get_ch() {
            '#' => self.read_comment_simd(),
            '"' | '\'' => self.read_quoted_content_simd(),
            _ if self.get_ch().is_alphabetic() || self.get_ch() == '_' => self.read_word_simd(),
            _ => {
                // Fall back to regular tokenization for special characters
                self.next_token()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_word_reading() {
        let mut lexer = Lexer::new("hello world");
        let token = lexer.read_word_simd();
        assert_eq!(token.kind, TokenKind::Word("hello".to_string()));
        assert_eq!(token.value, "hello");
    }

    #[test]
    fn test_simd_keyword_detection() {
        let mut lexer = Lexer::new("if then else fi");
        let token1 = lexer.read_word_simd();
        assert_eq!(token1.kind, TokenKind::If);

        lexer.skip_whitespace_simd();
        let token2 = lexer.read_word_simd();
        assert_eq!(token2.kind, TokenKind::Then);
    }

    #[test]
    fn test_simd_comment_reading() {
        let mut lexer = Lexer::new("# This is a comment\necho hello");
        let token = lexer.read_comment_simd();
        assert_eq!(token.kind, TokenKind::Comment);
        assert_eq!(token.value, "# This is a comment");
    }

    #[test]
    fn test_simd_quoted_string() {
        let mut lexer = Lexer::new("\"hello world\"");
        lexer.read_char(); // Move to the quote
        let token = lexer.read_quoted_content_simd();
        assert_eq!(token.kind, TokenKind::Word("hello world".to_string()));
    }

    #[test]
    fn test_simd_next_token() {
        let mut lexer = Lexer::new("echo \"hello\" # comment");

        let token1 = lexer.next_token_simd();
        assert_eq!(token1.kind, TokenKind::Word("echo".to_string()));

        let token2 = lexer.next_token_simd();
        assert_eq!(token2.kind, TokenKind::Quote);

        let token3 = lexer.next_token_simd();
        assert_eq!(token3.kind, TokenKind::Word("hello".to_string()));
    }

    #[test]
    fn test_simd_performance_large_input() {
        let large_input = "echo hello world ".repeat(1000);
        let mut lexer = Lexer::new(&large_input);

        let mut token_count = 0;
        loop {
            let token = lexer.next_token_simd();
            if token.kind == TokenKind::EOF {
                break;
            }
            token_count += 1;
        }

        assert!(token_count > 0);
    }
}
