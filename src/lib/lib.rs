//! This library contains representation of the linked network of cells.
//!
//! The possible way for traverse of the network
//!
//! For example:

pub mod message;
pub mod agent;
pub mod agent_pool;
pub use self::message::{Message};
pub use self::agent::{Agent};
pub use self::agent_pool::{AgentPool};
