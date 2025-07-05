/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

//! Integration layer for the new environment system with the existing interpreter

use crate::flash::env::Environment;
use crate::interpreter::Interpreter;

/// Extension trait to add environment functionality to the existing Interpreter
pub trait EnvironmentIntegration {
    /// Initialize the interpreter with the new environment system
    fn init_environment(&mut self);

    /// Get a variable using the new environment system
    fn get_env_var(&self, name: &str) -> Option<String>;

    /// Set a variable using the new environment system
    fn set_env_var(&mut self, name: &str, value: String);

    /// Export a variable
    fn export_var(&mut self, name: &str, value: Option<String>);

    /// Push a new local scope (for functions)
    fn push_local_scope(&mut self);

    /// Pop the current local scope
    fn pop_local_scope(&mut self);

    /// Set positional parameters ($0, $1, etc.)
    fn set_positional_parameters(&mut self, params: Vec<String>);

    /// Update exit status
    fn update_exit_status(&mut self, status: i32);
}

impl EnvironmentIntegration for Interpreter {
    fn init_environment(&mut self) {
        // Create new environment
        let mut env = Environment::new();

        // Migrate existing variables to new system
        for (key, value) in &self.variables {
            env.set(key, value.clone());
        }

        // Set positional parameters from args
        if !self.args.is_empty() {
            env.set_positional_params(self.args.clone());
        }

        // Update exit status
        env.set_exit_status(self.last_exit_code);

        // Store the environment (we'll need to modify Interpreter struct to include this)
        // For now, this is a demonstration of how it would work
    }

    fn get_env_var(&self, name: &str) -> Option<String> {
        // For now, fallback to existing system
        // In the full implementation, this would use the Environment
        self.variables.get(name).cloned()
    }

    fn set_env_var(&mut self, name: &str, value: String) {
        // For now, update existing system
        // In the full implementation, this would use the Environment
        self.variables.insert(name.to_string(), value);
    }

    fn export_var(&mut self, name: &str, value: Option<String>) {
        if let Some(val) = value {
            self.set_env_var(name, val.clone());
            unsafe {
                std::env::set_var(name, val);
            }
        } else if let Some(existing) = self.get_env_var(name) {
            unsafe {
                std::env::set_var(name, existing);
            }
        }
    }

    fn push_local_scope(&mut self) {
        // In the full implementation, this would call env.push_scope()
        // For now, this is a placeholder
    }

    fn pop_local_scope(&mut self) {
        // In the full implementation, this would call env.pop_scope()
        // For now, this is a placeholder
    }

    fn set_positional_parameters(&mut self, params: Vec<String>) {
        self.args = params;
        // In the full implementation, this would call env.set_positional_params()
    }

    fn update_exit_status(&mut self, status: i32) {
        self.last_exit_code = status;
        self.variables.insert("?".to_string(), status.to_string());
        // In the full implementation, this would call env.set_exit_status()
    }
}

/// Helper functions for environment management
pub mod env_helpers {
    use super::*;

    /// Initialize shell with enhanced environment
    pub fn initialize_shell_environment() -> Environment {
        let mut env = Environment::new();

        // Load RC file if it exists
        if let Some(home) = env.get("HOME") {
            let rc_file = format!("{home}/.flashrc");
            if std::path::Path::new(&rc_file).exists() {
                // TODO: Load and execute RC file
                env.set("FLASH_RC_LOADED", "1".to_string());
            }
        }

        // Set up interactive mode detection
        if atty::is(atty::Stream::Stdin) {
            env.shell_flags.push('i'); // interactive
            env.set("PS1", "flash$ ".to_string());
        }

        env
    }

    /// Create a subshell environment
    pub fn create_subshell_environment(parent: &Environment) -> Environment {
        let mut subshell = parent.clone();

        // Increment SHLVL
        if let Some(shlvl) = subshell.get("SHLVL") {
            if let Ok(level) = shlvl.parse::<i32>() {
                subshell.set_exported("SHLVL", (level + 1).to_string());
            }
        }

        // Update BASH_SUBSHELL equivalent
        if let Some(subshell_level) = subshell.get("FLASH_SUBSHELL") {
            if let Ok(level) = subshell_level.parse::<i32>() {
                subshell.set("FLASH_SUBSHELL", (level + 1).to_string());
            }
        } else {
            subshell.set("FLASH_SUBSHELL", "1".to_string());
        }

        subshell
    }

    /// Set up function local environment
    pub fn setup_function_environment(env: &mut Environment, func_name: &str, args: Vec<String>) {
        env.push_scope();

        // Set function name
        env.set("FUNCNAME", func_name.to_string());

        // Set function arguments
        env.set_positional_params(args);
    }

    /// Clean up function environment
    pub fn cleanup_function_environment(env: &mut Environment) {
        env.pop_scope();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_creation() {
        let env = Environment::new();

        // Check that basic variables are set
        assert!(env.has("SHELL"));
        assert!(env.has("FLASH_VERSION"));
        assert!(env.has("PWD"));
        assert_eq!(env.get("SHELL"), Some("flash".to_string()));
    }

    #[test]
    fn test_variable_scoping() {
        let mut env = Environment::new();

        // Set a variable in global scope
        env.set("TEST_VAR", "global".to_string());
        assert_eq!(env.get("TEST_VAR"), Some("global".to_string()));

        // Push local scope and override
        env.push_scope();
        env.set("TEST_VAR", "local".to_string());
        assert_eq!(env.get("TEST_VAR"), Some("local".to_string()));

        // Pop scope, should return to global
        env.pop_scope();
        assert_eq!(env.get("TEST_VAR"), Some("global".to_string()));
    }

    #[test]
    fn test_special_parameters() {
        let env = Environment::new();

        // Check special parameters exist
        assert!(env.get("?").is_some());
        assert!(env.get("$").is_some());
        assert!(env.get("#").is_some());
    }

    #[test]
    fn test_positional_parameters() {
        let mut env = Environment::new();

        env.set_positional_params(vec![
            "flash".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
        ]);

        assert_eq!(env.get("0"), Some("flash".to_string()));
        assert_eq!(env.get("1"), Some("arg1".to_string()));
        assert_eq!(env.get("2"), Some("arg2".to_string()));
        assert_eq!(env.get("#"), Some("2".to_string()));
    }

    #[test]
    fn test_export_functionality() {
        let mut env = Environment::new();

        env.set("TEST_EXPORT", "value".to_string());
        env.export("TEST_EXPORT");

        // Check that it's in the system environment
        assert_eq!(std::env::var("TEST_EXPORT").ok(), Some("value".to_string()));

        // Clean up
        unsafe {
            std::env::remove_var("TEST_EXPORT");
        }
    }
}
