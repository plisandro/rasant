/// Creates a [List][`crate::Value::List`] from types which can be cast to a collection of [Scalar][`crate::Scalar`]s.
#[macro_export]
macro_rules! list {
    // list!(scalar_collection)
    ($ss:expr) => {
	    rasant::Value::from(&rasant::Scalar::to_array($ss))
	};

    // list!(scalar_a, scalar_b, ...)
	($( $s:expr ),*) => {
	    rasant::Value::List(&[
			$( rasant::Scalar::from($s) ),*
		])
	};
}

/// Creates a [Map][`crate::Value::Map`] from key/value types which can be cast to collections of [Scalar][`crate::Scalar`]s.
#[macro_export]
macro_rules! map {
    // list!(keys_collection, values_collection)
    ($keys:expr, $values:expr) => {
	    rasant::Value::Map(
	        &rasant::Scalar::to_array($keys),
	        &rasant::Scalar::to_array($values)
		)
	};

    // map!(scalar_key_a => scalar_value_a, scalar_key_b => scalar_value_b, ...)
	($( $key:expr => $val:expr ),*) => {
	    rasant::Value::Map(
			&[
			    $( rasant::Scalar::from($key) ),*
			],
			&[
			    $( rasant::Scalar::from($val) ),*
			],
		)
	};
}

/// Sets a number of attributes for a given [logger][`crate::Logger`].
#[macro_export]
macro_rules! set {
    // set!(logger, key=value...)
	($logger:ident, $( $key:ident = $value:expr ),*) => {
	    $(
			_ = $logger.set(stringify!($key), $value);
		)*
	};
}

/// Logs an update with [trace][`crate::Level::Trace`] level.
#[macro_export]
macro_rules! trace {
    // trace!(logger, msg, key=value...)
	($logger:ident, $msg:expr) => {
		_ = $logger.trace($msg);
	};

    // trace!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		_ = $logger.trace_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from(&$value))
			),*
		]);
	};
}

/// Logs an update with [debug][`crate::Level::Debug`] level.
#[macro_export]
macro_rules! debug {
    // debug!(logger, msg)
	($logger:ident, $msg:expr) => {
		_ = $logger.debug($msg);
	};

    // debug!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		_ = $logger.debug_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from(&$value))
			),*
		]);
	};
}

/// Logs an update with [info][`crate::Level::Info`] level.
#[macro_export]
macro_rules! info {
    // info!(logger, msg)
	($logger:ident, $msg:expr) => {
		_ = $logger.info($msg);
	};

    // info!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		_ = $logger.info_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from(&$value))
			),*
		]);
	};
}

/// Logs an update with [warning][`crate::Level::Warning`] level.
#[macro_export]
macro_rules! warn {
    // warn!(logger, msg)
	($logger:ident, $msg:expr) => {
		_ = $logger.warn($msg);
	};

    // warn!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		_ = $logger.warn_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from(&$value))
			),*
		]);
	};
}

/// Logs an update with [error][`crate::Level::Error`] level.
#[macro_export]
macro_rules! error {
    // error!(logger, msg)
	($logger:ident, $msg:expr) => {
		_ = $logger.err($msg);
	};

    // error!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		_ = $logger.err_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from(&$value))
			),*
		]);
	};

    // error!(logger, msg, error, key=value...)
	($logger:ident, $error:expr, $msg:expr) => {
		_ =$ logger.error($error, $msg);
	};

    // error!(logger, msg, error, key=value...)
	($logger:ident, $error:expr, $msg:expr, $( $key:ident = $value:expr ),*) => {
		_ = $logger.error_with($error, $msg, [
		    $(
				(stringify!($key), rasant::Value::from(&$value))
			),*
		]);
	};
}

/// Logs an update with [fatal][`crate::Level::Fatal`] level.
#[macro_export]
macro_rules! fatal {
    // fatal!(logger, msg)
	($logger:ident, $msg:expr) => {
		_ = $logger.fatal($msg);
	};

    // fatal!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		_ = $logger.fatal_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from(&$value))
			),*
		]);
	};
}

/// Logs an update with [panic][`crate::Level::Panic`] level.
#[macro_export]
macro_rules! panic {
	($logger:ident, $msg:expr) => {
		_ = $logger.panic($msg);
	};

    // panic!(logger, msg, key=value...)
	($logger:ident, $msg:expr, $( $key:ident = $value:expr ),*) => {
		_ = $logger.panic_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from(&$value))
			),*
		]);
	};
}
