use std::sync::Arc;
use std::sync::Mutex;

use crate::sink::Sink;

/// An Arc'ed & Mutex'ed reference to a shared log [`Sink`].
pub type SinkRef = Arc<Mutex<Box<dyn Sink + Send>>>;
