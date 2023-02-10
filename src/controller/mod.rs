use crate::Result;
pub mod dynamic;
pub mod pod;

pub use pod::*;

trait Controller {
    fn resync(&self) -> Result<()>;
}
