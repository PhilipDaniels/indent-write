#![cfg_attr(not(feature = "std"), no_std)]

//! Simple indentation adapters for [`io::Write`][std::io::Write],
//! [`fmt::Write`][std::fmt::Write], and [`Display`][std::fmt::Display]. Each
//! adapter wraps a writer or writable object, and inserts an indentation at
//! the front of each non-empty line.
//!
//! See [`fmt::IndentWriter`], [`io::IndentWriter`], and
//! [`indentable::Indentable`] for examples.

pub mod fmt;
pub mod indentable;

#[cfg(feature = "std")]
pub mod io;
