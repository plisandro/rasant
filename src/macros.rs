#[macro_export]
macro_rules! trace {
    // trace!(msg, key=value...)
	($logger:ident, $msg:literal) => {
		$logger.trace($msg);
	};

    // trace!(msg, key=value...)
	($logger:ident, $msg:literal, $( $key:literal = $value:expr ),*) => {
		$logger.trace_with($msg, [
		    $(
				($key, $value.to_value())
			),*
		]);
	};
}

#[macro_export]
macro_rules! debug {
    // debug!(msg)
	($logger:ident, $msg:literal) => {
		$logger.debug($msg);
	};

    // debug!(msg, key=value...)
	($logger:ident, $msg:literal, $( $key:literal = $value:expr ),*) => {
		$logger.debug_with($msg, [
		    $(
				($key, $value.to_value())
			),*
		]);
	};
}

#[macro_export]
macro_rules! info {
    // info!(msg)
	($logger:ident, $msg:literal) => {
		$logger.info($msg);
	};

    // info!(msg, key=value...)
	($logger:ident, $msg:literal, $( $key:literal = $value:expr ),*) => {
		$logger.info_with($msg, [
		    $(
				($key, $value.to_value())
			),*
		]);
	};
}

#[macro_export]
macro_rules! warn {
    // warn!(msg)
	($logger:ident, $msg:literal) => {
		$logger.warn($msg);
	};

    // warn!(msg, key=value...)
	($logger:ident, $msg:literal, $( $key:literal = $value:expr ),*) => {
		$logger.warn_with($msg, [
		    $(
				($key, $value.to_value())
			),*
		]);
	};
}

#[macro_export]
macro_rules! error {
    // error!(msg)
	($logger:ident, $msg:literal) => {
		$logger.err($msg);
	};

    // error!(msg, key=value...)
	($logger:ident, $msg:literal, $( $key:literal = $value:expr ),*) => {
		$logger.err_with($msg, [
		    $(
				($key, $value.to_value())
			),*
		]);
	};

    // error!(msg, error, key=value...)
	($logger:ident, $error: expr, $msg:literal) => {
		$logger.error($error, $msg);
	};

    // error!(msg, error, key=value...)
	($logger:ident, $error: expr, $msg:literal, $( $key:literal = $value:expr ),*) => {
		$logger.error_with($error, $msg, [
		    $(
				($key, $value.to_value())
			),*
		]);
	};
}

#[macro_export]
macro_rules! fatal {
    // fatal!(msg)
	($logger:ident, $msg:literal) => {
		$logger.fatal($msg);
	};

    // fatal!(msg, key=value...)
	($logger:ident, $msg:literal, $( $key:literal = $value:expr ),*) => {
		$logger.fatal_with($msg, [
		    $(
				($key, $value.to_value())
			),*
		]);
	};
}

#[macro_export]
macro_rules! panic {
    // panic!(msg)
	($logger:ident, $msg:literal) => {
		$logger.panic($msg);
	};

    // panic!(msg, key=value...)
	($logger:ident, $msg:literal, $( $key:literal = $value:expr ),*) => {
		$logger.panic_with($msg, [
		    $(
				($key, $value.to_value())
			),*
		]);
	};
}
