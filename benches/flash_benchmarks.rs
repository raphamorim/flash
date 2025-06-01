/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use flash::lexer::Lexer;
use flash::parser::Parser;
// use flash::simd::{find_quotes, find_special_chars, find_whitespace};

#[cfg(feature = "formatter")]
use flash::formatter::{Formatter, FormatterConfig, ShellVariant};

// fn simd_benchmarks(c: &mut Criterion) {
//     let mut group = c.benchmark_group("simd");

//     let simple_text = b"hello world | grep test";
//     let complex_text = b"if [ \"$USER\" = \"root\" ]; then echo 'Running as root'; fi";
//     let large_text = "echo hello world | grep test && ls -la || exit 1; ".repeat(100);
//     let large_text_bytes = large_text.as_bytes();

//     // Benchmark SIMD character finding functions
//     group.bench_with_input(
//         BenchmarkId::new("find_special_chars", "simple"),
//         &simple_text,
//         |b, input| b.iter(|| black_box(find_special_chars(*black_box(input), 0))),
//     );

//     group.bench_with_input(
//         BenchmarkId::new("find_special_chars", "complex"),
//         &complex_text,
//         |b, input| b.iter(|| black_box(find_special_chars(*black_box(input), 0))),
//     );

//     group.bench_with_input(
//         BenchmarkId::new("find_special_chars", "large"),
//         &large_text_bytes,
//         |b, input| b.iter(|| black_box(find_special_chars(black_box(input), 0))),
//     );

//     group.bench_with_input(
//         BenchmarkId::new("find_whitespace", "simple"),
//         &simple_text,
//         |b, input| b.iter(|| black_box(find_whitespace(*black_box(input), 0))),
//     );

//     group.bench_with_input(
//         BenchmarkId::new("find_quotes", "complex"),
//         &complex_text,
//         |b, input| b.iter(|| black_box(find_quotes(*black_box(input), 0))),
//     );

//     group.finish();
// }

// fn lexer_simd_benchmarks(c: &mut Criterion) {
//     let mut group = c.benchmark_group("lexer_simd");

//     let simple_command = "echo hello world";
//     let complex_command = r#"
//         if [ "$USER" = "root" ]; then
//             echo "Running as root"
//             for file in /etc/*.conf; do
//                 if [ -f "$file" ]; then
//                     echo "Processing $file"
//                     cat "$file" | grep -E "^[^#]" | sort
//                 fi
//             done
//         elif [ -n "$HOME" ]; then
//             cd "$HOME" && ls -la
//         else
//             echo "Unknown user environment"
//         fi
//     "#;

//     let large_script = "echo hello world | grep test && ls -la || exit 1; ".repeat(500);

//     // Compare regular lexer vs SIMD lexer
//     group.bench_with_input(
//         BenchmarkId::new("regular_lexer", "simple"),
//         &simple_command,
//         |b, input| {
//             b.iter(|| {
//                 let mut lexer = Lexer::new(black_box(input));
//                 let mut tokens = Vec::new();
//                 loop {
//                     let token = lexer.next_token();
//                     if token.kind == flash::lexer::TokenKind::EOF {
//                         break;
//                     }
//                     tokens.push(token);
//                 }
//                 black_box(tokens)
//             })
//         },
//     );

//     group.bench_with_input(
//         BenchmarkId::new("simd_lexer", "simple"),
//         &simple_command,
//         |b, input| {
//             b.iter(|| {
//                 let mut lexer = Lexer::new(black_box(input));
//                 let mut tokens = Vec::new();
//                 loop {
//                     let token = lexer.next_token_simd();
//                     if token.kind == flash::lexer::TokenKind::EOF {
//                         break;
//                     }
//                     tokens.push(token);
//                 }
//                 black_box(tokens)
//             })
//         },
//     );

//     group.bench_with_input(
//         BenchmarkId::new("regular_lexer", "large"),
//         &large_script,
//         |b, input| {
//             b.iter(|| {
//                 let mut lexer = Lexer::new(black_box(input));
//                 let mut tokens = Vec::new();
//                 loop {
//                     let token = lexer.next_token();
//                     if token.kind == flash::lexer::TokenKind::EOF {
//                         break;
//                     }
//                     tokens.push(token);
//                 }
//                 black_box(tokens)
//             })
//         },
//     );

//     group.bench_with_input(
//         BenchmarkId::new("simd_lexer", "large"),
//         &large_script,
//         |b, input| {
//             b.iter(|| {
//                 let mut lexer = Lexer::new(black_box(input));
//                 let mut tokens = Vec::new();
//                 loop {
//                     let token = lexer.next_token_simd();
//                     if token.kind == flash::lexer::TokenKind::EOF {
//                         break;
//                     }
//                     tokens.push(token);
//                 }
//                 black_box(tokens)
//             })
//         },
//     );

//     group.finish();
// }

fn lexer_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer");

    let simple_command = "echo hello world";
    let complex_command = r#"
        if [ "$USER" = "root" ]; then
            echo "Running as root"
            for file in /etc/*.conf; do
                if [ -f "$file" ]; then
                    echo "Processing $file"
                    cat "$file" | grep -E "^[^#]" | sort
                fi
            done
        elif [ -n "$HOME" ]; then
            cd "$HOME" && ls -la
        else
            echo "Unknown user environment"
        fi
    "#;

    let pipeline_command = "cat /etc/passwd | grep root | cut -d: -f1 | sort | uniq";
    let variable_expansion = r#"echo "Hello $USER, today is $(date +%Y-%m-%d)""#;

    group.bench_with_input(
        BenchmarkId::new("simple_command", simple_command.len()),
        &simple_command,
        |b, input| {
            b.iter(|| {
                let mut lexer = Lexer::new(black_box(input));
                let mut tokens = Vec::new();
                loop {
                    let token = lexer.next_token();
                    if token.kind == flash::lexer::TokenKind::EOF {
                        break;
                    }
                    tokens.push(token);
                }
                black_box(tokens)
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("complex_conditional", complex_command.len()),
        &complex_command,
        |b, input| {
            b.iter(|| {
                let mut lexer = Lexer::new(black_box(input));
                let mut tokens = Vec::new();
                loop {
                    let token = lexer.next_token();
                    if token.kind == flash::lexer::TokenKind::EOF {
                        break;
                    }
                    tokens.push(token);
                }
                black_box(tokens)
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("pipeline", pipeline_command.len()),
        &pipeline_command,
        |b, input| {
            b.iter(|| {
                let mut lexer = Lexer::new(black_box(input));
                let mut tokens = Vec::new();
                loop {
                    let token = lexer.next_token();
                    if token.kind == flash::lexer::TokenKind::EOF {
                        break;
                    }
                    tokens.push(token);
                }
                black_box(tokens)
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("variable_expansion", variable_expansion.len()),
        &variable_expansion,
        |b, input| {
            b.iter(|| {
                let mut lexer = Lexer::new(black_box(input));
                let mut tokens = Vec::new();
                loop {
                    let token = lexer.next_token();
                    if token.kind == flash::lexer::TokenKind::EOF {
                        break;
                    }
                    tokens.push(token);
                }
                black_box(tokens)
            })
        },
    );

    group.finish();
}

fn parser_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    let simple_command = "echo hello world";
    let complex_command = r#"
        if [ "$USER" = "root" ]; then
            echo "Running as root"
            for file in /etc/*.conf; do
                if [ -f "$file" ]; then
                    echo "Processing $file"
                    cat "$file" | grep -E "^[^#]" | sort
                fi
            done
        elif [ -n "$HOME" ]; then
            cd "$HOME" && ls -la
        else
            echo "Unknown user environment"
        fi
    "#;

    let pipeline_command = "cat /etc/passwd | grep root | cut -d: -f1 | sort | uniq";
    let function_definition = r#"
        function backup_files() {
            local source_dir="$1"
            local backup_dir="$2"
            
            if [ ! -d "$source_dir" ]; then
                echo "Source directory does not exist"
                return 1
            fi
            
            mkdir -p "$backup_dir"
            cp -r "$source_dir"/* "$backup_dir"/
        }
    "#;

    group.bench_with_input(
        BenchmarkId::new("simple_command", simple_command.len()),
        &simple_command,
        |b, input| {
            b.iter(|| {
                let lexer = Lexer::new(black_box(input));
                let mut parser = Parser::new(lexer);
                black_box(parser.parse_script())
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("complex_conditional", complex_command.len()),
        &complex_command,
        |b, input| {
            b.iter(|| {
                let lexer = Lexer::new(black_box(input));
                let mut parser = Parser::new(lexer);
                black_box(parser.parse_script())
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("pipeline", pipeline_command.len()),
        &pipeline_command,
        |b, input| {
            b.iter(|| {
                let lexer = Lexer::new(black_box(input));
                let mut parser = Parser::new(lexer);
                black_box(parser.parse_script())
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("function_definition", function_definition.len()),
        &function_definition,
        |b, input| {
            b.iter(|| {
                let lexer = Lexer::new(black_box(input));
                let mut parser = Parser::new(lexer);
                black_box(parser.parse_script())
            })
        },
    );

    group.finish();
}

#[cfg(feature = "formatter")]
fn formatter_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("formatter");

    let config = FormatterConfig {
        indent_str: "    ".to_string(),
        shell_variant: ShellVariant::Bash,
        binary_next_line: false,
        switch_case_indent: true,
        space_redirects: true,
        keep_padding: false,
        function_next_line: false,
        never_split: false,
        format_if_needed: false,
    };

    let simple_command = "echo hello world";
    let complex_command = r#"
if [ "$USER" = "root" ]; then
echo "Running as root"
for file in /etc/*.conf; do
if [ -f "$file" ]; then
echo "Processing $file"
cat "$file" | grep -E "^[^#]" | sort
fi
done
elif [ -n "$HOME" ]; then
cd "$HOME" && ls -la
else
echo "Unknown user environment"
fi
    "#;

    let unformatted_script = r#"
function backup_files(){
local source_dir="$1"
local backup_dir="$2"
if [ ! -d "$source_dir" ];then
echo "Source directory does not exist"
return 1
fi
mkdir -p "$backup_dir"
cp -r "$source_dir"/* "$backup_dir"/
}
    "#;

    group.bench_with_input(
        BenchmarkId::new("simple_command", simple_command.len()),
        &simple_command,
        |b, input| {
            b.iter(|| {
                let mut formatter = Formatter::with_config(config.clone());
                black_box(formatter.format_str(black_box(input)))
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("complex_conditional", complex_command.len()),
        &complex_command,
        |b, input| {
            b.iter(|| {
                let mut formatter = Formatter::with_config(config.clone());
                black_box(formatter.format_str(black_box(input)))
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("unformatted_script", unformatted_script.len()),
        &unformatted_script,
        |b, input| {
            b.iter(|| {
                let mut formatter = Formatter::with_config(config.clone());
                black_box(formatter.format_str(black_box(input)))
            })
        },
    );

    group.finish();
}

fn end_to_end_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");

    let test_scripts = vec![
        ("simple", "echo hello && ls -la"),
        (
            "medium",
            r#"
            for i in {1..10}; do
                echo "Processing item $i"
                if [ $i -eq 5 ]; then
                    echo "Halfway there!"
                fi
            done
        "#,
        ),
        (
            "complex",
            r#"
            #!/bin/bash
            
            function process_logs() {
                local log_dir="$1"
                local output_file="$2"
                
                if [ ! -d "$log_dir" ]; then
                    echo "Error: Log directory '$log_dir' does not exist" >&2
                    return 1
                fi
                
                echo "Processing logs in $log_dir..."
                
                find "$log_dir" -name "*.log" -type f | while read -r logfile; do
                    echo "Processing: $logfile"
                    
                    # Extract errors and warnings
                    grep -E "(ERROR|WARN)" "$logfile" | \
                        sed 's/^/['"$(basename "$logfile")"'] /' >> "$output_file"
                    
                    # Count lines processed
                    line_count=$(wc -l < "$logfile")
                    echo "Processed $line_count lines from $(basename "$logfile")"
                done
                
                echo "Log processing complete. Results saved to $output_file"
            }
            
            # Main execution
            if [ $# -ne 2 ]; then
                echo "Usage: $0 <log_directory> <output_file>"
                exit 1
            fi
            
            process_logs "$1" "$2"
        "#,
        ),
    ];

    for (name, script) in test_scripts {
        group.bench_with_input(BenchmarkId::new("lex_parse", name), &script, |b, input| {
            b.iter(|| {
                let lexer = Lexer::new(black_box(input));
                let mut parser = Parser::new(lexer);
                black_box(parser.parse_script())
            })
        });

        #[cfg(feature = "formatter")]
        group.bench_with_input(
            BenchmarkId::new("lex_parse_format", name),
            &script,
            |b, input| {
                b.iter(|| {
                    let config = FormatterConfig {
                        indent_str: "    ".to_string(),
                        shell_variant: ShellVariant::Bash,
                        binary_next_line: false,
                        switch_case_indent: true,
                        space_redirects: true,
                        keep_padding: false,
                        function_next_line: false,
                        never_split: false,
                        format_if_needed: false,
                    };
                    let mut formatter = Formatter::with_config(config);
                    black_box(formatter.format_str(black_box(input)))
                })
            },
        );
    }

    group.finish();
}

#[cfg(feature = "formatter")]
criterion_group!(
    benches,
    // simd_benchmarks,
    // lexer_simd_benchmarks,
    lexer_benchmarks,
    parser_benchmarks,
    formatter_benchmarks,
    end_to_end_benchmarks
);

#[cfg(not(feature = "formatter"))]
criterion_group!(
    benches,
    simd_benchmarks,
    lexer_simd_benchmarks,
    lexer_benchmarks,
    parser_benchmarks,
    end_to_end_benchmarks
);

criterion_main!(benches);
