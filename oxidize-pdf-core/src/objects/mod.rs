mod array;
mod dictionary;
mod primitive;
mod stream;

pub use array::Array;
pub use dictionary::Dictionary;
pub use primitive::{Object, ObjectId};
pub use stream::Stream;

// Type alias for compatibility
pub type ObjectReference = ObjectId;
