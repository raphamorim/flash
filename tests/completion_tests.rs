/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use flash::completion::{CompletionContext, CompletionSystem};
use flash::interpreter::Interpreter;
use std::env;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_completion_system_comprehensive() {
    let mut system = CompletionSystem::new();

    // Test command completion
    let context = CompletionSystem::parse_context("gi", 2);
    let completions = system.complete(&context);
    assert!(
        completions.iter().any(|c| c == "git"),
        "Should complete git"
    );

    // Test git subcommand completion
    let context = CompletionSystem::parse_context("git ", 4);
    let completions = system.complete(&context);
    assert!(completions.contains(&"add".to_string()));
    assert!(completions.contains(&"commit".to_string()));
    assert!(completions.contains(&"push".to_string()));

    // Test git partial subcommand completion
    let context = CompletionSystem::parse_context("git ad", 6);
    let completions = system.complete(&context);
    assert!(completions.contains(&"add".to_string()));
    assert!(!completions.contains(&"commit".to_string()));
}

#[test]
fn test_file_completion_in_temp_directory() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create test files
    fs::write(temp_path.join("test_file.txt"), "content").unwrap();
    fs::write(temp_path.join("another_file.rs"), "code").unwrap();
    fs::create_dir(temp_path.join("test_dir")).unwrap();

    // Change to temp directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_path).unwrap();

    let system = CompletionSystem::new();

    // Test file completion
    let completions = system.complete_files("test");
    assert!(completions.iter().any(|c| c.contains("test_file.txt")));
    assert!(completions.iter().any(|c| c.contains("test_dir/")));

    // Test directory-only completion
    let completions = system.complete_directories("");
    assert!(completions.iter().any(|c| c == "test_dir/"));
    assert!(!completions.iter().any(|c| c.contains("test_file.txt")));

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_cd_completion_integration() {
    let mut interpreter = Interpreter::new();

    // Test cd completion returns only directories
    let (_suffixes, full_names) = interpreter.generate_completions("cd ", 3);

    // All completions should be directories (end with /)
    for completion in &full_names {
        assert!(
            completion.ends_with('/'),
            "CD completion '{}' should be a directory",
            completion
        );
    }
}

#[test]
fn test_variable_completion_integration() {
    let mut interpreter = Interpreter::new();

    // Add test variables
    interpreter
        .variables
        .insert("TEST_VAR1".to_string(), "value1".to_string());
    interpreter
        .variables
        .insert("TEST_VAR2".to_string(), "value2".to_string());
    interpreter
        .variables
        .insert("OTHER_VAR".to_string(), "value3".to_string());

    // Test variable completion with prefix
    let (_suffixes, full_names) = interpreter.generate_completions("echo $TEST_", 11);

    // Should complete both TEST_VAR1 and TEST_VAR2
    assert!(
        full_names.iter().any(|c| c == "$TEST_VAR1"),
        "Should complete TEST_VAR1"
    );
    assert!(
        full_names.iter().any(|c| c == "$TEST_VAR2"),
        "Should complete TEST_VAR2"
    );
    assert!(
        !full_names.iter().any(|c| c == "$OTHER_VAR"),
        "Should not complete OTHER_VAR"
    );
}

#[test]
fn test_pipe_completion() {
    let mut interpreter = Interpreter::new();

    // Test completion after pipe should suggest commands
    let (_suffixes, full_names) = interpreter.generate_completions("ls | e", 6);

    // Should complete commands starting with 'e'
    assert!(
        full_names.iter().any(|c| c == "echo"),
        "Should complete echo after pipe"
    );
}

#[test]
fn test_completion_with_multiple_spaces() {
    let mut interpreter = Interpreter::new();

    // Test completion with multiple spaces
    let (suffixes, full_names) = interpreter.generate_completions("git  add  ", 10);

    // Should handle multiple spaces gracefully
    assert!(suffixes.len() == full_names.len());
}

#[test]
fn test_completion_performance_stress() {
    let mut interpreter = Interpreter::new();

    use std::time::Instant;
    let start = Instant::now();

    // Test many completions
    for i in 0..1000 {
        let input = format!("git {}", i % 10);
        let _ = interpreter.generate_completions(&input, input.len());
    }

    let duration = start.elapsed();
    assert!(
        duration.as_millis() < 5000,
        "Completion stress test should complete in reasonable time"
    );
}

#[test]
fn test_completion_with_special_characters() {
    let mut interpreter = Interpreter::new();

    // Test completion with special characters in path
    let test_cases = [
        "ls file-with-dashes",
        "cat file_with_underscores",
        "echo file.with.dots",
    ];

    for test_case in &test_cases {
        let (suffixes, full_names) = interpreter.generate_completions(test_case, test_case.len());
        // Should not crash
        assert!(suffixes.len() == full_names.len());
    }
}

#[test]
fn test_completion_context_edge_cases() {
    // Test context parsing with edge cases
    let test_cases = [
        ("", 0),
        (" ", 1),
        ("  ", 2),
        ("cmd", 0),
        ("cmd", 1),
        ("cmd", 2),
        ("cmd", 3),
        ("cmd ", 4),
        ("cmd  ", 5),
    ];

    for (input, pos) in &test_cases {
        let context = CompletionSystem::parse_context(input, *pos);
        // Should not crash and should have valid structure
        assert!(context.cword <= context.words.len());
        assert_eq!(context.line, *input);
        assert_eq!(context.point, *pos);
    }
}

#[test]
fn test_git_branch_completion_mock() {
    let system = CompletionSystem::new();

    // Test git checkout completion (branch completion)
    let context = CompletionContext {
        line: "git checkout ".to_string(),
        point: 13,
        words: vec!["git".to_string(), "checkout".to_string()],
        cword: 2,
        current_word: "".to_string(),
        prev_word: "checkout".to_string(),
    };

    let completions = system.complete_git(&context);
    // Should attempt branch completion (may be empty if no git repo)
    // Just verify it doesn't crash
    let _ = completions;
}

#[test]
fn test_ssh_hostname_completion() {
    let system = CompletionSystem::new();

    let context = CompletionContext {
        line: "ssh ".to_string(),
        point: 4,
        words: vec!["ssh".to_string()],
        cword: 1,
        current_word: "".to_string(),
        prev_word: "ssh".to_string(),
    };

    let completions = system.complete_ssh(&context);
    // Should return hostname completions (may be empty depending on system)
    // Just verify it doesn't crash
    let _ = completions;
}

#[test]
fn test_kill_process_completion() {
    let system = CompletionSystem::new();

    let context = CompletionContext {
        line: "kill ".to_string(),
        point: 5,
        words: vec!["kill".to_string()],
        cword: 1,
        current_word: "".to_string(),
        prev_word: "kill".to_string(),
    };

    let completions = system.complete_kill(&context);
    // Should return process completions
    // Just verify it doesn't crash and all completions are non-empty
    for completion in &completions {
        assert!(!completion.is_empty(), "Completion should not be empty");
    }
}

#[test]
fn test_man_page_completion() {
    let system = CompletionSystem::new();

    let context = CompletionContext {
        line: "man ".to_string(),
        point: 4,
        words: vec!["man".to_string()],
        cword: 1,
        current_word: "".to_string(),
        prev_word: "man".to_string(),
    };

    let completions = system.complete_man(&context);
    // Should return man page completions
    // Just verify it doesn't crash
    let _ = completions;
}

#[test]
fn test_completion_system_extensibility() {
    let mut system = CompletionSystem::new();

    // Test that we can add new completion entries
    use flash::completion::CompletionEntry;
    use std::collections::HashMap;

    let new_entry = CompletionEntry {
        function: "_custom_complete".to_string(),
        action: "".to_string(),
        options: HashMap::new(),
        o_options: vec!["nospace".to_string()],
    };

    system
        .command_completions
        .insert("custom".to_string(), new_entry);

    // Verify it was added
    assert!(system.command_completions.contains_key("custom"));

    // Test completion for the new command
    let context = CompletionSystem::parse_context("custom ", 7);
    let completions = system.complete(&context);

    // Should call the custom function (which returns empty since it's not implemented)
    assert_eq!(completions, Vec::<String>::new());
}

#[test]
fn test_completion_with_tilde_expansion() {
    let system = CompletionSystem::new();

    // Test tilde expansion in file completion
    if let Ok(_home) = env::var("HOME") {
        let completions = system.complete_files("~/");

        // Should handle tilde expansion
        for completion in &completions {
            if !completion.is_empty() {
                assert!(
                    completion.starts_with("~/"),
                    "Tilde completion should preserve tilde prefix"
                );
            }
        }
    }
}

#[test]
fn test_completion_sorting_and_deduplication() {
    let system = CompletionSystem::new();

    // Test that completions are properly sorted and deduplicated
    let completions = system.complete_commands("");

    // Should be sorted
    let mut sorted_completions = completions.clone();
    sorted_completions.sort();
    assert_eq!(
        completions, sorted_completions,
        "Completions should be sorted"
    );

    // Should be deduplicated
    let mut dedup_completions = completions.clone();
    dedup_completions.dedup();
    assert_eq!(
        completions, dedup_completions,
        "Completions should be deduplicated"
    );
}
