//! PDF actions according to ISO 32000-1 Chapter 12.6
//!
//! Actions allow PDF documents to trigger various behaviors like navigation,
//! opening URIs, or executing named actions.

mod action;
mod goto_action;
mod launch_action;
mod named_action;
mod uri_action;

pub use action::{Action, ActionDictionary, ActionType};
pub use goto_action::{GoToAction, RemoteGoToAction};
pub use launch_action::{LaunchAction, LaunchParameters};
pub use named_action::{NamedAction, StandardNamedAction};
pub use uri_action::{UriAction, UriActionFlags};
