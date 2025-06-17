//! Matrixon A2A Protocol Library
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Purpose: Agent-to-Agent (A2A) protocol implementation for Matrixon
//! License: MIT

pub mod error;
pub mod state;
pub mod crypto;
pub mod transport;
pub mod message;
pub mod protocol;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
} 
