trace-error
===========

Extensions to Rust's error system to automatically include backtraces
to the exact location an error originates.

Consider this a more lightweight and less macro-based alternative to `error_chain` and similar crates. This crate
does not take care of actually defining the errors and their varieties, but only focuses on a thin container
for holding the errors and a backtrace to their origin.

`Trace` and `TraceResult` should usually be used in place of `Result` using the macros
`throw!`, `try_throw!`, and `try_rethrow!`

Although the `?` syntax was just introduced, `trace-error` is not yet compatible with it until the `Carrier` trait is stabilized. As a result,
all instances of `try!` and `?` should be replaced with `try_throw!` if you intend to use this crate to its fullest. However, the `?` operator
can be used for `Result<_, Trace<E>>` when the return value is also a `Result` using `Trace<E>`, just because `From` is implemented for types for itself.

If the `Trace` being returned in a result does **NOT** contain the same error type, but they are convertible, use `try_rethrow!` to convert the inner error type.

Example:

```rust
#[macro_use]
extern crate trace_error;

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;
use std::fs::File;

use trace_error::*;

pub type MyResultType<T> = TraceResult<T, MyErrorType>;

#[derive(Debug)]
pub enum MyErrorType {
    Io(io::Error),
    ErrorOne,
    ErrorTwo,
    //etc
}

impl Display for MyErrorType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.description())
    }
}

impl Error for MyErrorType {
    fn description(&self) -> &str {
        match *self {
            MyErrorType::Io(ref err) => err.description(),
            MyErrorType::ErrorOne => "Error One",
            MyErrorType::ErrorTwo => "Error Two",
        }
    }
}

impl From<io::Error> for MyErrorType {
    fn from(err: io::Error) -> MyErrorType {
        MyErrorType::Io(err)
    }
}

fn basic() -> MyResultType<i32> {
    //Something may throw
    throw!(MyErrorType::ErrorOne);

    // Or return an Ok value
    Ok(42)
}

fn example() -> MyResultType<()> {
    // Note the use of try_rethrow! for TraceResult results
    let meaning = try_rethrow!(basic());

    // Prints 42 if `basic` succeeds
    println!("{}", meaning);

    // Note the use of try_throw! for non-TraceResult results
    let some_file = try_throw!(File::open("Cargo.toml"));

    Ok(())
}

fn main() {
    match example() {
        Ok(_) => println!("Success!"),
        // Here, err is the Trace<E>, which can be printed normally,
        // showing both the error and the backtrace.
        Err(err) => panic!("Error: {}", err)
    }
}
```