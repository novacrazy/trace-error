//! Extensions to Rust's error system to automatically include backtraces
//! to the exact location an error originates.
//!
//! Consider this a more lightweight and less macro-based alternative to `error_chain` and similar crates. This crate
//! does not take care of actually defining the errors and their varieties, but only focuses on a thin container
//! for holding the errors and a backtrace to their origin.
//!
//! `Trace` and `TraceResult` should usually be used in place of `Result` using the macros
//! `throw!`, `try_throw!`, and `try_rethrow!`
//!
//! Although the `?` syntax was just introduced, `trace-error` is not yet compatible with it until the `Carrier` trait is stabilized. As a result,
//! all instances of `try!` and `?` should be replaced with `try_throw!` if you intend to use this crate to its fullest. However, the `?` operator
//! can be used for `Result<_, Trace<E>>` when the return value is also a `Result` using `Trace<E>`, just because `From` is implemented for types for itself.
//!
//! If the `Trace` being returned in a result does **NOT** contain the same error type, but they are convertible, use `try_rethrow!` to convert the inner error type.
//!
//! Additionally, if you must use the `Result<T, Trace<E>>` directly instead of immediately returning it, you can use the `trace_error!` macro to create it with the desired error value.
//!
//! Example:
//!
//! ```
//! #[macro_use]
//! extern crate trace_error;
//!
//! use std::error::Error;
//! use std::fmt::{Display, Formatter, Result as FmtResult};
//! use std::io;
//! use std::fs::File;
//!
//! use trace_error::TraceResult;
//!
//! pub type MyResultType<T> = TraceResult<T, MyErrorType>;
//!
//! #[derive(Debug)]
//! pub enum MyErrorType {
//!     Io(io::Error),
//!     ErrorOne,
//!     ErrorTwo,
//!     //etc
//! }
//!
//! impl Display for MyErrorType {
//!     fn fmt(&self, f: &mut Formatter) -> FmtResult {
//!         write!(f, "{}", self.description())
//!     }
//! }
//!
//! impl Error for MyErrorType {
//!     fn description(&self) -> &str {
//!         match *self {
//!             MyErrorType::Io(ref err) => err.description(),
//!             MyErrorType::ErrorOne => "Error One",
//!             MyErrorType::ErrorTwo => "Error Two",
//!         }
//!     }
//! }
//!
//! impl From<io::Error> for MyErrorType {
//!     fn from(err: io::Error) -> MyErrorType {
//!         MyErrorType::Io(err)
//!     }
//! }
//!
//! fn basic() -> MyResultType<i32> {
//!     Ok(42)
//! }
//!
//! fn example() -> MyResultType<()> {
//!     // Note the use of try_rethrow! for TraceResult results
//!     let meaning = try_rethrow!(basic());
//!
//!     // Prints 42 if `basic` succeeds
//!     println!("{}", meaning);
//!
//!     // Note the use of try_throw! for non-TraceResult results
//!     let some_file = try_throw!(File::open("Cargo.toml"));
//!
//!     Ok(())
//! }
//!
//! fn main() {
//!     match example() {
//!         Ok(_) => println!("Success!"),
//!         // Here, err is the Trace<E>, which can be printed normally,
//!         // showing both the error and the backtrace.
//!         Err(err) => println!("Error: {}", err)
//!     }
//! }
//! ```
#![allow(unknown_lints, inline_always)]
#![deny(missing_docs)]

extern crate backtrace as bt;

pub mod backtrace;

use std::error::Error;
use std::ops::Deref;
use std::fmt::{Display, Formatter, Result as FmtResult};

use backtrace::{BacktraceFmt, DefaultBacktraceFmt, SourceBacktrace};

/// Alias to aid in usage with `Result`
pub type TraceResult<T, E> = Result<T, Trace<E>>;

/// Trace error that encapsulates a backtrace alongside an error value.
///
/// Trace itself does not implement `Error`, so they cannot be nested.
#[derive(Debug)]
pub struct Trace<E: Error> {
    error: E,
    backtrace: Box<SourceBacktrace>,
}

impl<E: Error> Trace<E> {
    /// Creates a new `Trace` from the given error and backtrace
    #[inline]
    pub fn new(error: E, backtrace: Box<SourceBacktrace>) -> Trace<E> {
        Trace { error: error, backtrace: backtrace }
    }

    /// Consume self and return the inner error value
    #[inline]
    pub fn into_error(self) -> E {
        self.error
    }

    /// Get a reference to the inner backtrace
    #[inline]
    pub fn backtrace(&self) -> &SourceBacktrace {
        &*self.backtrace
    }

    /// Format the error and backtrace
    pub fn format<Fmt: BacktraceFmt>(&self, header: bool, reverse: bool) -> String {
        format!("{}\n{}", self.error, self.backtrace.format::<Fmt>(header, reverse))
    }

    /// Convert the inner error of type `E` into type `O`
    #[inline]
    pub fn convert<O: Error>(self) -> Trace<O> where O: From<E> {
        Trace {
            error: From::from(self.error),
            backtrace: self.backtrace
        }
    }
}

unsafe impl<E: Error> Send for Trace<E> where E: Send {}

impl<E: Error> Deref for Trace<E> {
    type Target = E;

    #[inline]
    fn deref(&self) -> &E {
        &self.error
    }
}

impl<E: Error> Display for Trace<E> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.format::<DefaultBacktraceFmt>(true, false))
    }
}

/// Creates a new `Result::Err(Trace<E>)` and immediately returns it.
///
/// This relies on the return type of the function to
/// provide type inference for the `Result::Ok(T)` type.
#[macro_export]
macro_rules! throw {
    ($err:expr) => { return trace_error!($err) }
}

/// Like `try!`, but invokes `throw!` on the error value if it exists, converting it to `Result::Err(Trace<E>)`
///
/// Note that the backtrace will only go as far as the location this macro was invoked
///
/// This macro will try to call `From::from` on the error to convert it if necessary, just like `try!`
///
/// This relies on the return type of the function to
/// provide type inference for the `Result::Ok(T)` type.
#[macro_export]
macro_rules! try_throw {
    ($res:expr) => (match $res {
        ::std::result::Result::Ok(val) => val,
        ::std::result::Result::Err(err) => { throw!(err) }
    })
}

#[doc(hidden)]
#[inline(always)]
pub fn _assert_trace_result<T, E: Error>(res: TraceResult<T, E>) -> TraceResult<T, E> {
    res
}

/// Like `try_throw!`, but designed for `TraceResult`s, as it keeps the previous trace.
///
/// This macro will try to call `Trace::convert` on the trace to convert the inner error if necessary,
/// similarly to `try!`
///
/// This relies on the return type of the function to
/// provide type inference for the `Result::Ok(T)` type.
#[macro_export]
macro_rules! try_rethrow {
    ($res:expr) => (match $crate::_assert_trace_result($res) {
        ::std::result::Result::Ok(val) => val,
        ::std::result::Result::Err(err) => {
            return ::std::result::Result::Err(err.convert())
        }
    })
}

/// The core macro that creates the `Result::Err(Trace<E>)` value,
/// but does not return it immediately.
///
/// An optional second parameter can be given to indicate the type the `Result`
/// should be if type inference cannot determine it automatically.
#[macro_export]
macro_rules! trace_error {
    ($err:expr) => {
        ::std::result::Result::Err($crate::Trace::new(
            ::std::convert::From::from($err),
            ::std::boxed::Box::new($crate::backtrace::SourceBacktrace::new(line!(), file!()))
        ))
    };

    ($err:expr, $t:ty) => {
        ::std::convert::From::<$t>::from(::std::result::Result::Err($crate::Trace::new(
            ::std::convert::From::from($err),
            ::std::boxed::Box::new($crate::backtrace::SourceBacktrace::new(line!(), file!()))
        )))
    };
}
