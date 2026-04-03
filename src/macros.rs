#[macro_export]
macro_rules! set {
    // set!(logger, key=value...)
	($logger:ident, $( $key:ident = $value:expr ),*) => {
	    $(
			$logger.set_value(stringify!($key), rasant::Value::from($value));
		),*
	};
}

#[macro_export]
macro_rules! trace {
    // trace!(logger, msg, key=value...)
	($logger:ident, $msg:literal) => {
		$logger.trace($msg);
	};

    // trace!(logger, msg, key=value...)
	($logger:ident, $msg:literal, $( $key:ident = $value:expr ),*) => {
		$logger.trace_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

#[macro_export]
macro_rules! debug {
    // debug!(logger, msg)
	($logger:ident, $msg:literal) => {
		$logger.debug($msg);
	};

    // debug!(logger, msg, key=value...)
	($logger:ident, $msg:literal, $( $key:ident = $value:expr ),*) => {
		$logger.debug_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

#[macro_export]
macro_rules! info {
    // info!(logger, msg)
	($logger:ident, $msg:literal) => {
		$logger.info($msg);
	};

    // info!(logger, msg, key=value...)
	($logger:ident, $msg:literal, $( $key:ident = $value:expr ),*) => {
		$logger.info_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

#[macro_export]
macro_rules! warn {
    // warn!(logger, msg)
	($logger:ident, $msg:literal) => {
		$logger.warn($msg);
	};

    // warn!(logger, msg, key=value...)
	($logger:ident, $msg:literal, $( $key:ident = $value:expr ),*) => {
		$logger.warn_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

#[macro_export]
macro_rules! error {
    // error!(logger, msg)
	($logger:ident, $msg:literal) => {
		$logger.err($msg);
	};

    // error!(logger, msg, key=value...)
	($logger:ident, $msg:literal, $( $key:ident = $value:expr ),*) => {
		$logger.err_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};

    // error!(logger, msg, error, key=value...)
	($logger:ident, $error: expr, $msg:literal) => {
		$logger.error($error, $msg);
	};

    // error!(logger, msg, error, key=value...)
	($logger:ident, $error: expr, $msg:literal, $( $key:ident = $value:expr ),*) => {
		$logger.error_with($error, $msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

#[macro_export]
macro_rules! fatal {
    // fatal!(logger, msg)
	($logger:ident, $msg:literal) => {
		$logger.fatal($msg);
	};

    // fatal!(logger, msg, key=value...)
	($logger:ident, $msg:literal, $( $key:ident = $value:expr ),*) => {
		$logger.fatal_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}

#[macro_export]
macro_rules! panic {
    // panic!(logger, msg)
	($logger:ident, $msg:literal) => {
		$logger.panic($msg);
	};

    // panic!(logger, msg, key=value...)
	($logger:ident, $msg:literal, $( $key:ident = $value:expr ),*) => {
		$logger.panic_with($msg, [
		    $(
				(stringify!($key), rasant::Value::from($value))
			),*
		]);
	};
}
