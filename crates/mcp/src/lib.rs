//! writer-mcp: lets an AI assistant read and write WriterProgram project
//! folders. The same tools are offered two ways: as an MCP server
//! (`writer-mcp`, src/main.rs) and as shell commands (`writer`, [`cli`]).

mod check;
pub mod cli;
mod context;
pub mod guide;
mod manage;
pub mod path;
mod readings;
mod review;
mod state;
mod text;
pub mod tools;
