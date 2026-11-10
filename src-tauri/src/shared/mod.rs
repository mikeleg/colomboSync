//! What more than one slice uses: the domain types, storage, user-facing messages, path rules,
//! MirrorGuard and the like.

pub mod blocking;
pub mod copy_text;
pub mod domain;
pub mod elevation;
pub mod error_messages;
pub mod history_store;
pub mod mirror_guard;
pub mod path_rules;
pub mod storage;
pub mod sync_store;
