pub mod dynamic;
pub mod ns;
pub mod pod;
pub use ns::*;
pub use pod::*;

use crate::Result;

trait Controller {
    fn resync(&self) -> Result<()>;
}
