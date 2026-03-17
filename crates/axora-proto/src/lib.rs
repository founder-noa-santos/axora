//! AXORA Protocol Buffer Definitions
//!
//! This crate contains the generated Protocol Buffer code for the AXORA
//! multi-agent system. It provides both client and server implementations
//! for the gRPC services.

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod collective {
    pub mod v1 {
        //! Core collective service definitions
        include!(concat!(env!("OUT_DIR"), "/collective.v1.rs"));
    }
}

pub use collective::v1::*;

/// Re-export prost for downstream users
pub use prost;
/// Re-export tonic for downstream users
pub use tonic;
