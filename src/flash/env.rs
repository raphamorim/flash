/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use std::collections::HashMap;
use std::{env, fs, io, path::PathBuf, process};

/// Variable data types supported by the shell
#[derive(Debug, Clone)]
pub enum VariableValue {
    String(String),
    Array(Vec<String>),
    AssocArray(HashMap<String, String>),
}

impl VariableValue {
    pub fn as_string(&self) -> String {
        match self {
            VariableValue::String(s) => s.clone(),
            VariableValue::Array(arr) => arr.join(" "),
            VariableValue::AssocArray(map) => map.values().cloned().collect::<Vec<_>>().join(" "),
        }
    }

    pub fn as_array(&self) -> Vec<String> {
        match self {
            VariableValue::String(s) => vec![s.clone()],
            VariableValue::Array(arr) => arr.clone(),
            VariableValue::AssocArray(map) => map.values().cloned().collect(),
        }
    }
}

/// Variable flags (readonly, export, etc.)
#[derive(Debug, Clone, Default)]
pub struct VariableFlags {
    pub readonly: bool,
    pub export: bool,
    pub integer: bool,
    pub array: bool,
    pub assoc: bool,
}

/// A shell variable with value and metadata
#[derive(Debug, Clone)]
pub struct Variable {
    pub value: VariableValue,
    pub flags: VariableFlags,
}

impl Variable {
    pub fn new_string(value: String) -> Self {
        Self {
            value: VariableValue::String(value),
            flags: VariableFlags::default(),
        }
    }

    pub fn new_array(values: Vec<String>) -> Self {
        Self {
            value: VariableValue::Array(values),
            flags: VariableFlags {
                array: true,
                ..Default::default()
            },
        }
    }

    pub fn new_exported(value: String) -> Self {
        Self {
            value: VariableValue::String(value),
            flags: VariableFlags {
                export: true,
                ..Default::default()
            },
        }
    }
}

/// Enhanced environment system with layered scoping
#[derive(Debug, Clone)]
pub struct Environment {
    /// Layered variable storage (local scopes)
    pub layers: Vec<HashMap<String, Variable>>,
    /// Special shell parameters ($?, $$, $!, etc.)
    pub special_params: HashMap<String, String>,
    /// Positional parameters ($0, $1, $2, ...)
    pub positional_params: Vec<String>,
    /// Shell options and flags
    pub shell_flags: String,
    /// Exit status of last command
    pub exit_status: i32,
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub ppid: Option<u32>,
    /// Current working directory
    pub pwd: Option<PathBuf>,
    /// Previous working directory
    pub oldpwd: Option<PathBuf>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        let mut env = Self {
            layers: vec![HashMap::new()],
            special_params: HashMap::new(),
            positional_params: vec!["flash".to_string()],
            shell_flags: "hB".to_string(), // h=hashall, B=brace_expand
            exit_status: 0,
            pid: process::id(),
            ppid: None,
            pwd: None,
            oldpwd: None,
        };

        env.initialize();
        env
    }

    /// Initialize the environment with system variables and shell defaults
    pub fn initialize(&mut self) {
        self.load_system_environment();
        self.set_shell_variables();
        self.set_special_parameters();
        self.initialize_pwd();
    }

    /// Load environment variables from the system
    fn load_system_environment(&mut self) {
        // Try to load from /proc/self/environ first (Linux/Unix)
        if let Ok(vars) = load_env_from_proc() {
            for (key, value) in vars {
                self.set_exported(&key, value);
            }
        } else {
            // Fallback to std::env
            for (key, value) in env::vars() {
                self.set_exported(&key, value);
            }
        }
    }

    /// Set up shell-specific variables
    fn set_shell_variables(&mut self) {
        let version = env!("CARGO_PKG_VERSION");
        let target_arch =
            std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "unknown".to_string());
        let target_vendor =
            std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_else(|_| "unknown".to_string());
        let target_os =
            std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "unknown".to_string());
        let machtype = format!("{}-{}-{}", target_arch, target_vendor, target_os);

        // Shell identification
        self.set_exported("SHELL", "flash".to_string());
        self.set_exported("FLASH_VERSION", version.to_string());
        self.set_exported("MACHTYPE", machtype);
        self.set_exported("HOSTTYPE", target_arch);
        self.set_exported("OSTYPE", target_os);

        // Shell level
        let shlvl = self
            .get("SHLVL")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
            + 1;
        self.set_exported("SHLVL", shlvl.to_string());

        // Default prompts
        if !self.has("PS1") {
            self.set("PS1", "flash$ ".to_string());
        }
        if !self.has("PS2") {
            self.set("PS2", "> ".to_string());
        }
        self.set("PS4", "+ ".to_string());

        // History settings
        if let Some(home) = self.get("HOME") {
            if !self.has("HISTFILE") {
                self.set("HISTFILE", format!("{}/.flash_history", home));
            }
        }
        if !self.has("HISTSIZE") {
            self.set("HISTSIZE", "1000".to_string());
        }
        if !self.has("HISTFILESIZE") {
            self.set("HISTFILESIZE", "2000".to_string());
        }

        // IFS (Internal Field Separator)
        if !self.has("IFS") {
            self.set("IFS", " \t\n".to_string());
        }

        // PATH enhancement for macOS
        #[cfg(target_os = "macos")]
        {
            if let Some(path) = self.get("PATH") {
                if !path.contains("/opt/homebrew/bin") {
                    self.set_exported("PATH", format!("/opt/homebrew/bin:{}", path));
                }
            }
        }
    }

    /// Set up special shell parameters
    fn set_special_parameters(&mut self) {
        self.special_params.insert("?".to_string(), "0".to_string());
        self.special_params
            .insert("$".to_string(), self.pid.to_string());
        self.special_params.insert("#".to_string(), "0".to_string());
        self.special_params.insert("*".to_string(), String::new());
        self.special_params.insert("@".to_string(), String::new());
        self.special_params
            .insert("-".to_string(), self.shell_flags.clone());
        self.special_params.insert("!".to_string(), "0".to_string());
        self.special_params.insert("_".to_string(), String::new());
    }

    /// Initialize PWD and OLDPWD
    fn initialize_pwd(&mut self) {
        if let Ok(current_dir) = env::current_dir() {
            self.pwd = Some(current_dir.clone());
            self.set_exported("PWD", current_dir.to_string_lossy().to_string());
        }
    }

    /// Push a new local scope
    pub fn push_scope(&mut self) {
        self.layers.push(HashMap::new());
    }

    /// Pop the current local scope
    pub fn pop_scope(&mut self) {
        if self.layers.len() > 1 {
            self.layers.pop();
        }
    }

    /// Set a variable in the current scope
    pub fn set(&mut self, name: &str, value: String) {
        if let Some(current_layer) = self.layers.last_mut() {
            current_layer.insert(name.to_string(), Variable::new_string(value));
        }
    }

    /// Set an exported variable
    pub fn set_exported(&mut self, name: &str, value: String) {
        if let Some(current_layer) = self.layers.last_mut() {
            current_layer.insert(name.to_string(), Variable::new_exported(value.clone()));
            unsafe {
                env::set_var(name, &value);
            }
        }
    }

    /// Set an array variable
    pub fn set_array(&mut self, name: &str, values: Vec<String>) {
        if let Some(current_layer) = self.layers.last_mut() {
            current_layer.insert(name.to_string(), Variable::new_array(values));
        }
    }

    /// Get a variable value as string
    pub fn get(&self, name: &str) -> Option<String> {
        // Check special parameters first
        if let Some(value) = self.special_params.get(name) {
            return Some(value.clone());
        }

        // Check positional parameters
        if let Ok(index) = name.parse::<usize>() {
            return self.positional_params.get(index).cloned();
        }

        // Search through layers from most recent to oldest
        for layer in self.layers.iter().rev() {
            if let Some(var) = layer.get(name) {
                return Some(var.value.as_string());
            }
        }

        None
    }

    /// Check if a variable exists
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Unset a variable
    pub fn unset(&mut self, name: &str) {
        for layer in &mut self.layers {
            if let Some(var) = layer.remove(name) {
                if var.flags.export {
                    unsafe {
                        env::remove_var(name);
                    }
                }
                break;
            }
        }
    }

    /// Export an existing variable
    pub fn export(&mut self, name: &str) {
        for layer in self.layers.iter_mut().rev() {
            if let Some(var) = layer.get_mut(name) {
                var.flags.export = true;
                unsafe {
                    env::set_var(name, &var.value.as_string());
                }
                return;
            }
        }
    }

    /// Set exit status and update $?
    pub fn set_exit_status(&mut self, status: i32) {
        self.exit_status = status;
        self.special_params
            .insert("?".to_string(), status.to_string());
    }

    /// Set positional parameters
    pub fn set_positional_params(&mut self, params: Vec<String>) {
        self.positional_params = params;
        self.special_params.insert(
            "#".to_string(),
            (self.positional_params.len().saturating_sub(1)).to_string(),
        );

        // Update $* and $@
        let args = if self.positional_params.len() > 1 {
            self.positional_params[1..].join(" ")
        } else {
            String::new()
        };
        self.special_params.insert("*".to_string(), args.clone());
        self.special_params.insert("@".to_string(), args);
    }

    /// Change directory and update PWD/OLDPWD
    pub fn change_directory(&mut self, new_dir: PathBuf) -> io::Result<()> {
        let old_pwd = self.pwd.clone();
        env::set_current_dir(&new_dir)?;

        self.oldpwd = old_pwd;
        self.pwd = Some(new_dir.clone());

        self.set_exported("PWD", new_dir.to_string_lossy().to_string());
        if let Some(old) = &self.oldpwd {
            self.set_exported("OLDPWD", old.to_string_lossy().to_string());
        }

        Ok(())
    }

    /// Get all exported variables for command execution
    pub fn get_exported_vars(&self) -> HashMap<String, String> {
        let mut exported = HashMap::new();

        for layer in &self.layers {
            for (name, var) in layer {
                if var.flags.export {
                    exported.insert(name.clone(), var.value.as_string());
                }
            }
        }

        exported
    }
}

pub fn load_env_from_proc() -> io::Result<HashMap<String, String>> {
    let mut variables = HashMap::new();

    // Read from /proc/self/environ (Linux/Unix only)
    let environ_data = fs::read("/proc/self/environ")?;

    // Split by null bytes and parse key=value pairs
    for env_pair in environ_data.split(|&b| b == 0) {
        if let Ok(env_str) = std::str::from_utf8(env_pair) {
            if let Some((key, value)) = env_str.split_once('=') {
                variables.insert(key.to_string(), value.to_string());
            }
        }
    }

    Ok(variables)
}

// unsafe extern "C" {
//     #[cfg(not(target_os = "windows"))]
//     static environ: *const *const c_char;
// }

// #[cfg(target_os = "windows")]
// unsafe extern "C" {
//     fn GetEnvironmentStringsA() -> *mut c_char;
//     fn FreeEnvironmentStringsA(env_block: *mut c_char) -> i32;
// }

// pub fn load_env() -> HashMap<String, String> {
//     let mut variables = HashMap::new();

//     #[cfg(not(target_os = "windows"))]
//     {
//         unsafe {
//             let mut env_ptr = environ;
//             while !(*env_ptr).is_null() {
//                 let c_str = CStr::from_ptr(*env_ptr);
//                 if let Ok(env_str) = c_str.to_str() {
//                     if let Some((key, value)) = env_str.split_once('=') {
//                         variables.insert(key.to_string(), value.to_string());
//                     }
//                 }
//                 env_ptr = env_ptr.add(1);
//             }
//         }
//     }

//     #[cfg(target_os = "windows")]
//     {
//         unsafe {
//             let env_block = GetEnvironmentStringsA();
//             if !env_block.is_null() {
//                 let mut current = env_block;
//                 loop {
//                     let c_str = CStr::from_ptr(current);
//                     let bytes = c_str.to_bytes();
//                     if bytes.is_empty() {
//                         break;
//                     }

//                     if let Ok(env_str) = std::str::from_utf8(bytes) {
//                         if let Some((key, value)) = env_str.split_once('=') {
//                             variables.insert(key.to_string(), value.to_string());
//                         }
//                     }

//                     current = current.add(bytes.len() + 1);
//                 }
//                 FreeEnvironmentStringsA(env_block);
//             }
//         }
//     }

//     variables
// }
