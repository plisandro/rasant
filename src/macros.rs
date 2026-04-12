/// Sets a number of attributes for a given [logger][`crate::Logger`].
#[macro_export]
macro_rules! set {
    // set!(logger, key=value...)
	($logger:ident, $( $key:ident = $value:expr ),*) => {
	    $(
			$logger.set_value(stringify!($key), rasant::Value::from($value));
		),*
	};
}

/// Logs an update with [trace][`crate::Level::Trace`] level.
#[macro_export]
macro_rules! trace {
    // trace!(logger, msg, key=value...)
	($logger:ident, $msg:expr) => {
		$logger.trace($msg);
	};

    // trace!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		$logger.trace_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

/// Logs an update with [debug][`crate::Level::Debug`] level.
#[macro_export]
macro_rules! debug {
    // debug!(logger, msg)
	($logger:ident, $msg:expr) => {
		$logger.debug($msg);
	};

    // debug!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		$logger.debug_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

/// Logs an update with [info][`crate::Level::Info`] level.
#[macro_export]
macro_rules! info {
    // info!(logger, msg)
	($logger:ident, $msg:expr) => {
		$logger.info($msg);
	};

    // info!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		$logger.info_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

/// Logs an update with [warning][`crate::Level::Warning`] level.
#[macro_export]
macro_rules! warn {
    // warn!(logger, msg)
	($logger:ident, $msg:expr) => {
		$logger.warn($msg);
	};

    // warn!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		$logger.warn_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

/// Logs an update with [error][`crate::Level::Error`] level.
#[macro_export]
macro_rules! error {
    // error!(logger, msg)
	($logger:ident, $msg:expr) => {
		$logger.err($msg);
	};

    // error!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		$logger.err_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};

    // error!(logger, msg, error, key=value...)
	($logger:ident, $error:expr, $msg:expr) => {
		$logger.error($error, $msg);
	};

    // error!(logger, msg, error, key=value...)
	($logger:ident, $error:expr, $msg:expr, $( $key:ident = $value:expr ),*) => {
		$logger.error_with($error, $msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

/// Logs an update with [fatal][`crate::Level::Fatal`] level.
#[macro_export]
macro_rules! fatal {
    // fatal!(logger, msg)
	($logger:ident, $msg:expr) => {
		$logger.fatal($msg);
	};

    // fatal!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		$logger.fatal_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

/// Logs an update with [panic][`crate::Level::Panic`] level.
#[macro_export]
macro_rules! panic {
	($logger:ident, $msg:expr) => {
		$logger.panic($msg);
	};

    // panic!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		$logger.panic_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}
