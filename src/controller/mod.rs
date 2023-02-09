use crate::EventHandler;
pub mod dynamic;
pub mod pod;

trait Controller: EventHandler {}
