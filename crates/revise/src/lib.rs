//! Prototype of the revise-mode style checker (docs/revise-engine.md).

pub mod hangul;
pub mod morph;
pub mod rules;
pub mod text;

pub use morph::Analyzer;
pub use rules::{Doc, Fix, Genre, Issue, Names, Profile, Severity, analyze, check};
