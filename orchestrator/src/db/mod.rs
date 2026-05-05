// SPDX-License-Identifier: AGPL-3.0-only

pub mod entities;
pub mod migrations;

// Re-export for backwards compatibility
pub use migrations as migration;
