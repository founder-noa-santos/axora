//! OPENAKTA Protocol Buffer Definitions
//!
//! This crate contains the generated Protocol Buffer code for the OPENAKTA
//! multi-agent system. It provides both client and server implementations
//! for the gRPC services.
//!
//! # Generated Types
//!
//! - [`Agent`] - Agent definition
//! - [`Task`] - Task definition
//! - [`Message`] - Message definition
//! - [`AgentStatus`] - Agent status enum
//! - [`TaskStatus`] - Task status enum
//! - [`MessageType`] - Message type enum

// Note: Generated protobuf code doesn't have docs, so we allow missing_docs for the module
#![allow(missing_docs)]

pub mod collective {
    #[allow(missing_docs)]
    pub mod v1 {
        //! Core collective service definitions
        include!(concat!(env!("OUT_DIR"), "/collective.v1.rs"));
    }
}

pub mod mcp {
    #[allow(missing_docs)]
    pub mod v1 {
        //! MCP tool service definitions
        include!(concat!(env!("OUT_DIR"), "/mcp.v1.rs"));
    }
}

pub mod livingdocs {
    #[allow(missing_docs)]
    pub mod v1 {
        //! LivingDocs review queue + SSOT resolution (Plan 6)
        include!(concat!(env!("OUT_DIR"), "/livingdocs.v1.rs"));
    }
}

pub use collective::v1::*;
pub use mcp::v1 as mcp_v1;

/// Re-export prost for downstream users
pub use prost;
/// Re-export tonic for downstream users
pub use tonic;
