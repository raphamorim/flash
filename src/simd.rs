/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

//! SIMD optimizations for lexer and parser operations.
//!
//! This module provides vectorized implementations for common operations
//! like finding special characters, whitespace, and keywords in shell scripts.

use std::ptr;

/// Find the first occurrence of any character in a set of needles.
///
/// Returns the index of the first occurrence, or `haystack.len()` if not found.
/// This is optimized for finding shell special characters like quotes, operators, etc.
pub fn find_special_chars(haystack: &[u8], offset: usize) -> usize {
    unsafe {
        let beg = haystack.as_ptr();
        let end = beg.add(haystack.len());
        let it = beg.add(offset.min(haystack.len()));
        let it = find_special_chars_raw(it, end);
        it.offset_from_unsigned(beg)
    }
}

/// Find the first occurrence of whitespace characters.
///
/// Returns the index of the first whitespace, or `haystack.len()` if not found.
pub fn find_whitespace(haystack: &[u8], offset: usize) -> usize {
    unsafe {
        let beg = haystack.as_ptr();
        let end = beg.add(haystack.len());
        let it = beg.add(offset.min(haystack.len()));
        let it = find_whitespace_raw(it, end);
        it.offset_from_unsigned(beg)
    }
}

/// Find the first occurrence of quote characters (' or ").
///
/// Returns the index of the first quote, or `haystack.len()` if not found.
pub fn find_quotes(haystack: &[u8], offset: usize) -> usize {
    unsafe {
        let beg = haystack.as_ptr();
        let end = beg.add(haystack.len());
        let it = beg.add(offset.min(haystack.len()));
        let it = find_quotes_raw(it, end);
        it.offset_from_unsigned(beg)
    }
}

/// Find the first occurrence of newline characters.
///
/// Returns the index of the first newline, or `haystack.len()` if not found.
pub fn find_newline(haystack: &[u8], offset: usize) -> usize {
    unsafe {
        let beg = haystack.as_ptr();
        let end = beg.add(haystack.len());
        let it = beg.add(offset.min(haystack.len()));
        let it = find_newline_raw(it, end);
        it.offset_from_unsigned(beg)
    }
}

unsafe fn find_special_chars_raw(beg: *const u8, end: *const u8) -> *const u8 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    return unsafe { SPECIAL_CHARS_DISPATCH(beg, end) };

    #[cfg(target_arch = "aarch64")]
    return unsafe { find_special_chars_neon(beg, end) };

    #[allow(unreachable_code)]
    return unsafe { find_special_chars_fallback(beg, end) };
}

unsafe fn find_whitespace_raw(beg: *const u8, end: *const u8) -> *const u8 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    return unsafe { WHITESPACE_DISPATCH(beg, end) };

    #[cfg(target_arch = "aarch64")]
    return unsafe { find_whitespace_neon(beg, end) };

    #[allow(unreachable_code)]
    return unsafe { find_whitespace_fallback(beg, end) };
}

unsafe fn find_quotes_raw(beg: *const u8, end: *const u8) -> *const u8 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    return unsafe { QUOTES_DISPATCH(beg, end) };

    #[cfg(target_arch = "aarch64")]
    return unsafe { find_quotes_neon(beg, end) };

    #[allow(unreachable_code)]
    return unsafe { find_quotes_fallback(beg, end) };
}

unsafe fn find_newline_raw(beg: *const u8, end: *const u8) -> *const u8 {
    // Simple case - just look for '\n'
    unsafe {
        let mut it = beg;
        while !ptr::eq(it, end) {
            if *it == b'\n' {
                break;
            }
            it = it.add(1);
        }
        it
    }
}

// Fallback implementations
unsafe fn find_special_chars_fallback(mut beg: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        while !ptr::eq(beg, end) {
            let ch = *beg;
            // Shell special characters: | & ; ( ) < > $ " ' ` # = ! { } \
            if matches!(
                ch,
                b'|' | b'&'
                    | b';'
                    | b'('
                    | b')'
                    | b'<'
                    | b'>'
                    | b'$'
                    | b'"'
                    | b'\''
                    | b'`'
                    | b'#'
                    | b'='
                    | b'!'
                    | b'{'
                    | b'}'
                    | b'\\'
            ) {
                break;
            }
            beg = beg.add(1);
        }
        beg
    }
}

unsafe fn find_whitespace_fallback(mut beg: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        while !ptr::eq(beg, end) {
            let ch = *beg;
            if matches!(ch, b' ' | b'\t' | b'\n' | b'\r') {
                break;
            }
            beg = beg.add(1);
        }
        beg
    }
}

unsafe fn find_quotes_fallback(mut beg: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        while !ptr::eq(beg, end) {
            let ch = *beg;
            if ch == b'"' || ch == b'\'' {
                break;
            }
            beg = beg.add(1);
        }
        beg
    }
}

// x86/x86_64 SIMD implementations
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static mut SPECIAL_CHARS_DISPATCH: unsafe fn(*const u8, *const u8) -> *const u8 =
    special_chars_dispatch;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static mut WHITESPACE_DISPATCH: unsafe fn(*const u8, *const u8) -> *const u8 = whitespace_dispatch;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static mut QUOTES_DISPATCH: unsafe fn(*const u8, *const u8) -> *const u8 = quotes_dispatch;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn special_chars_dispatch(beg: *const u8, end: *const u8) -> *const u8 {
    let func = if is_x86_feature_detected!("avx2") {
        find_special_chars_avx2
    } else {
        find_special_chars_fallback
    };
    unsafe { SPECIAL_CHARS_DISPATCH = func };
    unsafe { func(beg, end) }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn whitespace_dispatch(beg: *const u8, end: *const u8) -> *const u8 {
    let func = if is_x86_feature_detected!("avx2") {
        find_whitespace_avx2
    } else {
        find_whitespace_fallback
    };
    unsafe { WHITESPACE_DISPATCH = func };
    unsafe { func(beg, end) }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn quotes_dispatch(beg: *const u8, end: *const u8) -> *const u8 {
    let func = if is_x86_feature_detected!("avx2") {
        find_quotes_avx2
    } else {
        find_quotes_fallback
    };
    unsafe { QUOTES_DISPATCH = func };
    unsafe { func(beg, end) }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn find_special_chars_avx2(mut beg: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::*;

        let mut remaining = end.offset_from_unsigned(beg);

        // Check for common special characters in groups
        let pipe_amp = _mm256_set1_epi8(b'|' as i8);
        let semicolon = _mm256_set1_epi8(b';' as i8);
        let paren_open = _mm256_set1_epi8(b'(' as i8);
        let paren_close = _mm256_set1_epi8(b')' as i8);
        let less_than = _mm256_set1_epi8(b'<' as i8);
        let greater_than = _mm256_set1_epi8(b'>' as i8);
        let dollar = _mm256_set1_epi8(b'$' as i8);
        let double_quote = _mm256_set1_epi8(b'"' as i8);
        let single_quote = _mm256_set1_epi8(b'\'' as i8);
        let backtick = _mm256_set1_epi8(b'`' as i8);
        let hash = _mm256_set1_epi8(b'#' as i8);
        let equals = _mm256_set1_epi8(b'=' as i8);
        let exclamation = _mm256_set1_epi8(b'!' as i8);
        let brace_open = _mm256_set1_epi8(b'{' as i8);
        let brace_close = _mm256_set1_epi8(b'}' as i8);
        let backslash = _mm256_set1_epi8(b'\\' as i8);
        let ampersand = _mm256_set1_epi8(b'&' as i8);

        while remaining >= 32 {
            let v = _mm256_loadu_si256(beg as *const _);

            // Check for all special characters
            let match1 = _mm256_or_si256(
                _mm256_or_si256(
                    _mm256_cmpeq_epi8(v, pipe_amp),
                    _mm256_cmpeq_epi8(v, ampersand),
                ),
                _mm256_or_si256(
                    _mm256_cmpeq_epi8(v, semicolon),
                    _mm256_cmpeq_epi8(v, paren_open),
                ),
            );

            let match2 = _mm256_or_si256(
                _mm256_or_si256(
                    _mm256_cmpeq_epi8(v, paren_close),
                    _mm256_cmpeq_epi8(v, less_than),
                ),
                _mm256_or_si256(
                    _mm256_cmpeq_epi8(v, greater_than),
                    _mm256_cmpeq_epi8(v, dollar),
                ),
            );

            let match3 = _mm256_or_si256(
                _mm256_or_si256(
                    _mm256_cmpeq_epi8(v, double_quote),
                    _mm256_cmpeq_epi8(v, single_quote),
                ),
                _mm256_or_si256(_mm256_cmpeq_epi8(v, backtick), _mm256_cmpeq_epi8(v, hash)),
            );

            let match4 = _mm256_or_si256(
                _mm256_or_si256(
                    _mm256_cmpeq_epi8(v, equals),
                    _mm256_cmpeq_epi8(v, exclamation),
                ),
                _mm256_or_si256(
                    _mm256_cmpeq_epi8(v, brace_open),
                    _mm256_cmpeq_epi8(v, brace_close),
                ),
            );

            let final_match = _mm256_or_si256(
                _mm256_or_si256(match1, match2),
                _mm256_or_si256(
                    _mm256_or_si256(match3, match4),
                    _mm256_cmpeq_epi8(v, backslash),
                ),
            );

            let m = _mm256_movemask_epi8(final_match) as u32;

            if m != 0 {
                return beg.add(m.trailing_zeros() as usize);
            }

            beg = beg.add(32);
            remaining -= 32;
        }

        find_special_chars_fallback(beg, end)
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn find_whitespace_avx2(mut beg: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::*;

        let space = _mm256_set1_epi8(b' ' as i8);
        let tab = _mm256_set1_epi8(b'\t' as i8);
        let newline = _mm256_set1_epi8(b'\n' as i8);
        let carriage_return = _mm256_set1_epi8(b'\r' as i8);
        let mut remaining = end.offset_from_unsigned(beg);

        while remaining >= 32 {
            let v = _mm256_loadu_si256(beg as *const _);
            let a = _mm256_cmpeq_epi8(v, space);
            let b = _mm256_cmpeq_epi8(v, tab);
            let c = _mm256_cmpeq_epi8(v, newline);
            let d = _mm256_cmpeq_epi8(v, carriage_return);
            let result = _mm256_or_si256(_mm256_or_si256(a, b), _mm256_or_si256(c, d));
            let m = _mm256_movemask_epi8(result) as u32;

            if m != 0 {
                return beg.add(m.trailing_zeros() as usize);
            }

            beg = beg.add(32);
            remaining -= 32;
        }

        find_whitespace_fallback(beg, end)
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn find_quotes_avx2(mut beg: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::*;

        let double_quote = _mm256_set1_epi8(b'"' as i8);
        let single_quote = _mm256_set1_epi8(b'\'' as i8);
        let mut remaining = end.offset_from_unsigned(beg);

        while remaining >= 32 {
            let v = _mm256_loadu_si256(beg as *const _);
            let a = _mm256_cmpeq_epi8(v, double_quote);
            let b = _mm256_cmpeq_epi8(v, single_quote);
            let c = _mm256_or_si256(a, b);
            let m = _mm256_movemask_epi8(c) as u32;

            if m != 0 {
                return beg.add(m.trailing_zeros() as usize);
            }

            beg = beg.add(32);
            remaining -= 32;
        }

        find_quotes_fallback(beg, end)
    }
}

// ARM NEON implementations
#[cfg(target_arch = "aarch64")]
unsafe fn find_special_chars_neon(mut beg: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        use std::arch::aarch64::*;

        if end.offset_from_unsigned(beg) >= 16 {
            // For NEON, we'll check for the most common special characters
            let pipe = vdupq_n_u8(b'|');
            let amp = vdupq_n_u8(b'&');
            let semicolon = vdupq_n_u8(b';');
            let dollar = vdupq_n_u8(b'$');
            let double_quote = vdupq_n_u8(b'"');
            let single_quote = vdupq_n_u8(b'\'');

            loop {
                let v = vld1q_u8(beg as *const _);

                let match1 = vorrq_u8(vceqq_u8(v, pipe), vceqq_u8(v, amp));
                let match2 = vorrq_u8(vceqq_u8(v, semicolon), vceqq_u8(v, dollar));
                let match3 = vorrq_u8(vceqq_u8(v, double_quote), vceqq_u8(v, single_quote));

                let final_match = vorrq_u8(vorrq_u8(match1, match2), match3);

                // Convert to bitmask
                let m = vreinterpretq_u16_u8(final_match);
                let m = vshrn_n_u16(m, 4);
                let m = vreinterpret_u64_u8(m);
                let m = vget_lane_u64(m, 0);

                if m != 0 {
                    return beg.add(m.trailing_zeros() as usize >> 2);
                }

                beg = beg.add(16);
                if end.offset_from_unsigned(beg) < 16 {
                    break;
                }
            }
        }

        find_special_chars_fallback(beg, end)
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn find_whitespace_neon(mut beg: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        use std::arch::aarch64::*;

        if end.offset_from_unsigned(beg) >= 16 {
            let space = vdupq_n_u8(b' ');
            let tab = vdupq_n_u8(b'\t');
            let newline = vdupq_n_u8(b'\n');
            let carriage_return = vdupq_n_u8(b'\r');

            loop {
                let v = vld1q_u8(beg as *const _);
                let a = vceqq_u8(v, space);
                let b = vceqq_u8(v, tab);
                let c = vceqq_u8(v, newline);
                let d = vceqq_u8(v, carriage_return);
                let result = vorrq_u8(vorrq_u8(a, b), vorrq_u8(c, d));

                let m = vreinterpretq_u16_u8(result);
                let m = vshrn_n_u16(m, 4);
                let m = vreinterpret_u64_u8(m);
                let m = vget_lane_u64(m, 0);

                if m != 0 {
                    return beg.add(m.trailing_zeros() as usize >> 2);
                }

                beg = beg.add(16);
                if end.offset_from_unsigned(beg) < 16 {
                    break;
                }
            }
        }

        find_whitespace_fallback(beg, end)
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn find_quotes_neon(mut beg: *const u8, end: *const u8) -> *const u8 {
    unsafe {
        use std::arch::aarch64::*;

        if end.offset_from_unsigned(beg) >= 16 {
            let double_quote = vdupq_n_u8(b'"');
            let single_quote = vdupq_n_u8(b'\'');

            loop {
                let v = vld1q_u8(beg as *const _);
                let a = vceqq_u8(v, double_quote);
                let b = vceqq_u8(v, single_quote);
                let c = vorrq_u8(a, b);

                let m = vreinterpretq_u16_u8(c);
                let m = vshrn_n_u16(m, 4);
                let m = vreinterpret_u64_u8(m);
                let m = vget_lane_u64(m, 0);

                if m != 0 {
                    return beg.add(m.trailing_zeros() as usize >> 2);
                }

                beg = beg.add(16);
                if end.offset_from_unsigned(beg) < 16 {
                    break;
                }
            }
        }

        find_quotes_fallback(beg, end)
    }
}

/// Extension trait to add offset_from_unsigned method for compatibility
trait OffsetFromUnsigned<T> {
    fn offset_from_unsigned(self, origin: *const T) -> usize;
}

impl<T> OffsetFromUnsigned<T> for *const T {
    fn offset_from_unsigned(self, origin: *const T) -> usize {
        (self as usize - origin as usize) / std::mem::size_of::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_special_chars() {
        let input = b"hello world | grep test";
        assert_eq!(find_special_chars(input, 0), 12); // Position of '|'

        let input = b"echo $VAR";
        assert_eq!(find_special_chars(input, 0), 5); // Position of '$'

        let input = b"no special chars here";
        assert_eq!(find_special_chars(input, 0), input.len());
    }

    #[test]
    fn test_find_whitespace() {
        let input = b"hello world";
        assert_eq!(find_whitespace(input, 0), 5); // Position of space

        let input = b"hello\tworld";
        assert_eq!(find_whitespace(input, 0), 5); // Position of tab

        let input = b"nospaces";
        assert_eq!(find_whitespace(input, 0), input.len());
    }

    #[test]
    fn test_find_quotes() {
        let input = b"echo 'hello world'";
        assert_eq!(find_quotes(input, 0), 5); // Position of first quote

        let input = b"echo \"hello world\"";
        assert_eq!(find_quotes(input, 0), 5); // Position of first quote

        let input = b"no quotes here";
        assert_eq!(find_quotes(input, 0), input.len());
    }

    #[test]
    fn test_find_newline() {
        let input = b"line1\nline2";
        assert_eq!(find_newline(input, 0), 5); // Position of newline

        let input = b"no newline here";
        assert_eq!(find_newline(input, 0), input.len());
    }

    #[test]
    fn test_with_offset() {
        let input = b"echo 'hello' | grep 'world'";
        assert_eq!(find_quotes(input, 0), 5); // First quote
        assert_eq!(find_quotes(input, 6), 11); // Second quote (after offset)
        assert_eq!(find_special_chars(input, 0), 5); // First special char (quote)
        assert_eq!(find_special_chars(input, 12), 13); // Pipe after offset
    }
}
