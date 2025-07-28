//! Document structure elements including page trees, name trees, and outlines
//! according to ISO 32000-1

mod destination;
mod name_tree;
mod outline;
mod page_tree;

pub use destination::{Destination, DestinationType, PageDestination};
pub use name_tree::{NameTree, NameTreeNode, NamedDestinations};
pub use outline::{OutlineBuilder, OutlineItem, OutlineTree};
pub use page_tree::{PageTree, PageTreeBuilder, PageTreeNode};
