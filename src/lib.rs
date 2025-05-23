/*
 * Copyright (c) 2025 Raphael Amorim
 *
 * This file is part of flash, which is licensed
 * under GNU General Public License v3.0.
 */

pub mod lexer;
pub mod parser;

#[cfg(feature = "interpreter")]
mod flash;
#[cfg(feature = "formatter")]
pub mod formatter;
#[cfg(feature = "interpreter")]
pub mod interpreter;
