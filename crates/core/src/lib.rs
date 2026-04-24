//! Domain types, repository traits, and business logic for Roadboard.
//!
//! This crate is IO-free: no database, no HTTP, no filesystem access.
//! Repository traits are defined here; sqlx-backed implementations live
//! in `roadboard-storage`.
