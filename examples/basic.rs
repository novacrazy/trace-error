#[macro_use]
extern crate trace_error;

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;
use std::fs::File;

use trace_error::TraceResult;

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
    Ok(42)
}

fn example() -> MyResultType<()> {
    // Note the use of try_rethrow! for TraceResult results
    let meaning = try_rethrow!(basic());

    // Prints 42 if `basic` succeeds
    println!("{}", meaning);

    // Note the use of try_throw! for non-TraceResult results
    let _some_file = try_throw!(File::open("Cargos.toml"));

    Ok(())
}

fn main() {
    match example() {
        Ok(_) => println!("Success!"),
        // Here, err is the Trace<E>, which can be printed normally,
        // showing both the error and the backtrace.
        Err(err) => println!("Error: {}", err)
    }
}