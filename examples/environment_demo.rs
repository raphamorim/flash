/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

//! Example demonstrating the improved environment system

use flash::environment_integration::env_helpers;
use flash::flash::env::Environment;

fn main() {
    println!("=== Flash Shell Environment System Demo ===\n");

    // Create a new environment
    let mut env = Environment::new();

    println!("1. Basic Environment Variables:");
    println!("SHELL: {:?}", env.get("SHELL"));
    println!("FLASH_VERSION: {:?}", env.get("FLASH_VERSION"));
    println!("PWD: {:?}", env.get("PWD"));
    println!("MACHTYPE: {:?}", env.get("MACHTYPE"));
    println!("HOSTTYPE: {:?}", env.get("HOSTTYPE"));
    println!("OSTYPE: {:?}", env.get("OSTYPE"));
    println!();

    println!("2. Special Parameters:");
    println!("Exit status ($?): {:?}", env.get("?"));
    println!("Process ID ($$): {:?}", env.get("$"));
    println!("Number of args ($#): {:?}", env.get("#"));
    println!("All args ($*): {:?}", env.get("*"));
    println!("Shell flags ($-): {:?}", env.get("-"));
    println!();

    println!("3. Shell Level:");
    println!("SHLVL: {:?}", env.get("SHLVL"));
    println!();

    println!("4. History Configuration:");
    println!("HISTFILE: {:?}", env.get("HISTFILE"));
    println!("HISTSIZE: {:?}", env.get("HISTSIZE"));
    println!("HISTFILESIZE: {:?}", env.get("HISTFILESIZE"));
    println!();

    println!("5. Prompt Configuration:");
    println!("PS1: {:?}", env.get("PS1"));
    println!("PS2: {:?}", env.get("PS2"));
    println!("PS4: {:?}", env.get("PS4"));
    println!();

    println!("6. Field Separator:");
    println!("IFS: {:?}", env.get("IFS"));
    println!();

    println!("=== Testing Variable Scoping ===\n");

    // Test variable scoping
    env.set("GLOBAL_VAR", "global_value".to_string());
    println!("Before pushing scope:");
    println!("GLOBAL_VAR: {:?}", env.get("GLOBAL_VAR"));

    // Push a new scope (like entering a function)
    env.push_scope();
    env.set("LOCAL_VAR", "local_value".to_string());
    env.set("GLOBAL_VAR", "overridden_value".to_string());

    println!("\nInside local scope:");
    println!("LOCAL_VAR: {:?}", env.get("LOCAL_VAR"));
    println!("GLOBAL_VAR: {:?}", env.get("GLOBAL_VAR"));

    // Pop the scope (like exiting a function)
    env.pop_scope();

    println!("\nAfter popping scope:");
    println!("LOCAL_VAR: {:?}", env.get("LOCAL_VAR")); // Should be None
    println!("GLOBAL_VAR: {:?}", env.get("GLOBAL_VAR")); // Should be back to global
    println!();

    println!("=== Testing Export Functionality ===\n");

    // Test export
    env.set("TEST_EXPORT", "exported_value".to_string());
    env.export("TEST_EXPORT");
    println!("TEST_EXPORT (exported): {:?}", env.get("TEST_EXPORT"));
    println!(
        "In system environment: {:?}",
        std::env::var("TEST_EXPORT").ok()
    );
    println!();

    println!("=== Testing Array Variables ===\n");

    // Test array variables
    env.set_array(
        "TEST_ARRAY",
        vec![
            "item1".to_string(),
            "item2".to_string(),
            "item3".to_string(),
        ],
    );
    println!("Array variable set (would need special handling for display)");
    println!();

    println!("=== Testing Positional Parameters ===\n");

    // Test positional parameters
    env.set_positional_params(vec![
        "flash".to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]);

    println!("Positional parameters:");
    println!("$0: {:?}", env.get("0"));
    println!("$1: {:?}", env.get("1"));
    println!("$2: {:?}", env.get("2"));
    println!("$3: {:?}", env.get("3"));
    println!("$#: {:?}", env.get("#"));
    println!("$*: {:?}", env.get("*"));
    println!("$@: {:?}", env.get("@"));
    println!();

    println!("=== Testing Exit Status ===\n");

    env.set_exit_status(42);
    println!("Exit status set to 42");
    println!("$?: {:?}", env.get("?"));
    println!();

    println!("=== Testing Subshell Environment ===\n");

    // Create a subshell environment
    let subshell_env = env_helpers::create_subshell_environment(&env);
    println!("Subshell SHLVL: {:?}", subshell_env.get("SHLVL"));
    println!(
        "Subshell FLASH_SUBSHELL: {:?}",
        subshell_env.get("FLASH_SUBSHELL")
    );
    println!();

    println!("=== Testing Function Environment ===\n");

    // Simulate function call
    env_helpers::setup_function_environment(
        &mut env,
        "test_function",
        vec![
            "test_function".to_string(),
            "func_arg1".to_string(),
            "func_arg2".to_string(),
        ],
    );

    println!("Inside function:");
    println!("FUNCNAME: {:?}", env.get("FUNCNAME"));
    println!("$0: {:?}", env.get("0"));
    println!("$1: {:?}", env.get("1"));
    println!("$2: {:?}", env.get("2"));
    println!("$#: {:?}", env.get("#"));

    // Clean up function environment
    env_helpers::cleanup_function_environment(&mut env);

    println!("\nAfter function cleanup:");
    println!("FUNCNAME: {:?}", env.get("FUNCNAME"));
    println!();

    println!("=== Demo Complete ===");

    // Clean up exported variable
    unsafe {
        std::env::remove_var("TEST_EXPORT");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_demo() {
        // Just run the demo to make sure it doesn't panic
        main();
    }
}
