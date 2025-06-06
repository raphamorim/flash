use flash::interpreter::Interpreter;

fn main() {
    let interpreter = Interpreter::new();
    
    // Test what parse_alias_value returns for our stored alias
    let stored_value = "echo\\ hello\\ world";
    let parts = interpreter.parse_alias_value(stored_value);
    println!("Stored value: {:?}", stored_value);
    println!("Parsed parts: {:?}", parts);
    
    // Test what we expect
    let expected_value = "echo hello world";
    let expected_parts = interpreter.parse_alias_value(expected_value);
    println!("Expected value: {:?}", expected_value);
    println!("Expected parts: {:?}", expected_parts);
}
