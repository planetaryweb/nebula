//! # Nebula Email
//!
//! `nebula_email` will implement a form handler for
//! [`nebula`](https://crates.io/crates/nebula) that uses form values to
//! generate and send emails.
//!
//! ## In Progress
//!
//! This crate is a work in progress and, as of February 1st, 2020, only
//! contains code to parse a TOML configuration.

mod config;
mod sender;
//mod templates;
