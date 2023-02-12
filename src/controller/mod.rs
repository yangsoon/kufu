pub mod dynamic;
pub mod pod;
pub use pod::*;

use crate::Result;

trait Controller {
    fn resync(&self) -> Result<()>;
}
