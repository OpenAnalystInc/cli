//! /knowledge MOE (Mixture of Experts) system.
//!
//! Architecture:
//! 1. Intent Classifier — categorizes user query
//! 2. Learning DB — reads past learnings before query, writes feedback after
//! 3. Expert Router — selects the right agent/model per intent
//! 4. RAG Query — calls the KB backend API
//! 5. Synthesis — combines KB results + learnings + context
//! 6. Feedback Loop — user rates response, system extracts learnings
//!
//! SQLite schema at .openanalyst/knowledge.db stores:
//! - Query history with intent classification
//! - User feedback (rating + correction)
//! - Extracted learnings (positive + negative)
//! - Conversation context for persistent sessions

pub mod db;
pub mod intent;
pub mod expert;

pub use db::{CachedResult, LearningDb, normalize_query_hash};
pub use intent::{Intent, IntentClassifier};
pub use expert::{ExpertRouter, KnowledgeExpert};
