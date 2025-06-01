#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::formatter::Formatter;

    #[test]
    fn debug_brace_expansion() {
        let input = "echo {1..5}";
        println!("Input: {}", input);
        
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let result = parser.parse_script();
        
        println!("Parsed result: {:#?}", result);
        
        let mut formatter = Formatter::new();
        let output = formatter.format(&result);
        println!("Formatted output: {}", output);
    }
}
