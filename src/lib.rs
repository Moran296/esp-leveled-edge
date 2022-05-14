
mod debounce;
mod leveled_edge;

pub use debounce::*;
pub use leveled_edge::*;


// A simple crate that provides safer any edge interrupts for esp32, using alternating level interrupts (with debouncing)
// To make sure that the wrong state is not stuck after an edge interrupt was missed.











