use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;

use crate::queue::AsyncSinkOp;
use crate::sink::Sink;

/// An Arc'ed & Mutex'ed reference to a shared log [`Sink`].
pub type SinkRef = Arc<Mutex<Box<dyn Sink + Send>>>;

/// A sender channel for [`AsyncSinkOp`] async log operations.
pub type AsyncSinkSender = mpsc::Sender<AsyncSinkOp>;
