//! Small extensions to the `backtrace` crate
//!
//! This module defines a formatting API for formatting both inline and captured backtraces,
//! and a structure for holding file and line level captured backtraces.

use std::os::raw::c_void;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::path::Path;
use std::thread;
use std::mem;

use bt::{resolve, trace, Backtrace, Symbol, SymbolName, BacktraceSymbol};

/// Trait to define formatting for backtrace symbols
pub trait BacktraceFmt {
    /// Formats backtrace symbol components in some way
    fn format(count: u32, symbol: &Symbol) -> String;

    /// Same as `BacktraceFmt::format`, but accepts a captured `BacktraceSymbol` instead
    fn format_captured(count: u32, symbol: &BacktraceSymbol) -> String;
}

/// Default backtrace formatter that tries to resemble rustc panic backtraces somewhat
///
/// Example:
///
/// ```text
/// Stack backtrace for task "<main>" at line 47 of "examples\backtrace.rs":
///    0:     0x7ff703417000 - backtrace::test
///                         at E:\...\examples\backtrace.rs:47
///    1:     0x7ff703417120 - backtrace::main
///                         at E:\...\examples\backtrace.rs:53
///    2:     0x7ff70343bb10 - panic_unwind::__rust_maybe_catch_panic
///                         at C:\...\libpanic_unwind\lib.rs:98
///    3:     0x7ff70343b240 - std::rt::lang_start
///                         at C:\...\libstd\rt.rs:51
///    4:     0x7ff7034171a0 - main
///                         at <anonymous>
///    5:     0x7ff70344d61c - __scrt_common_main_seh
///                         at f:\...\exe_common.inl:253
///    6:     0x7ffead558350 - BaseThreadInitThunk
///                         at <anonymous>
/// ```
pub struct DefaultBacktraceFmt;

impl DefaultBacktraceFmt {
    fn real_format(count: u32,
                   name: Option<SymbolName>,
                   addr: Option<*mut c_void>,
                   filename: Option<&Path>,
                   lineno: Option<u32>) -> String {
        let ptr_width = mem::size_of::<usize>() * 2 + 2;

        let name = name.and_then(|name| { name.as_str() }).unwrap_or("<unknown>");

        let begin = format!("{:>4}: {:>4$p} - {:<}\n{:<5$}", count, addr.unwrap_or(0x0 as *mut _), name, "", ptr_width, ptr_width + 6);

        let end = if let Some(filename) = filename {
            if let Some(lineno) = lineno {
                format!("at {}:{}\n", filename.display(), lineno)
            } else {
                format!("at {}\n", filename.display())
            }
        } else if let Some(lineno) = lineno {
            format!("at <anonymous>:{}\n", lineno)
        } else {
            format!("at <anonymous>\n")
        };

        begin + end.as_str()
    }
}

impl BacktraceFmt for DefaultBacktraceFmt {
    #[inline]
    fn format(count: u32, symbol: &Symbol) -> String {
        DefaultBacktraceFmt::real_format(count, symbol.name(), symbol.addr(), symbol.filename(), symbol.lineno())
    }

    #[inline]
    fn format_captured(count: u32, symbol: &BacktraceSymbol) -> String {
        // Could just use format!("{:?}", symbol) since BacktraceSymbol has a debug format specifier, but eh, I like mine better
        DefaultBacktraceFmt::real_format(count, symbol.name(), symbol.addr(), symbol.filename(), symbol.lineno())
    }
}

/// Generates a formatted backtrace (via `Fmt` type) from here, but expects `line` and `file` to be where it was called from.
///
/// The actual call to `format_trace` and `trace` are ignored.
#[inline(never)]
pub fn format_trace<Fmt: BacktraceFmt>(header: bool, line: u32, file: &str) -> String {
    // Ignore `format_trace` and `backtrace::trace` calls, both of which are marked as #[inline(never)],
    // so they will always show up.
    const IGNORE_COUNT: u32 = 2;

    let mut traces = if header {
        format!("Stack backtrace for task \"<{}>\" at line {} of \"{}\":\n",
                thread::current().name().unwrap_or("unnamed"), line, file)
    } else {
        String::new()
    };

    let mut count = 0;

    trace(|frame| {
        if count < IGNORE_COUNT {
            count += 1;
        } else {
            let before = count;

            resolve(frame.ip(), |symbol| {
                traces += Fmt::format(count - IGNORE_COUNT, &symbol).as_str();

                count += 1;
            });

            // These will be equal if `resolve_cb` was not invoked
            if count == before {
                // If `symbol_address` doesn't work, oh well.
                resolve(frame.symbol_address(), |symbol| {
                    traces += Fmt::format(count - IGNORE_COUNT, &symbol).as_str();

                    count += 1;
                });
            }
        }

        // Always continue
        true
    });

    traces
}

/// Backtrace that also contains the exact line and file in which it originated from.
///
/// Usually created in a macro using the `line!()` and `file!()` macros
#[derive(Clone)]
pub struct SourceBacktrace {
    backtrace: Backtrace,
    line: u32,
    file: &'static str,
}

impl Debug for SourceBacktrace {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "SourceBacktrace {{\n    line: {},\n    file: {},\n    backtrace:\n{}}}", self.line, self.file, self.format::<DefaultBacktraceFmt>(false, false))
    }
}

impl SourceBacktrace {
    /// Create a new `SourceBacktrace` if you know the line and file
    pub fn new(line: u32, file: &'static str) -> SourceBacktrace {
        SourceBacktrace {
            backtrace: Backtrace::new(),
            line: line,
            file: file,
        }
    }

    /// Get a reference to the raw `Backtrace` instance
    #[inline]
    pub fn raw(&self) -> &Backtrace {
        &self.backtrace
    }

    /// Get the line at which this backtrace originated from
    #[inline]
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Get the file path (as a `&'static str`) in which this backtrace originated from
    #[inline]
    pub fn file(&self) -> &'static str {
        self.file
    }

    /// Format this backtrace with the given formatter and the given options
    pub fn format<Fmt: BacktraceFmt>(&self, header: bool, reverse: bool) -> String {
        // Ignore `backtrace::trace` call
        const IGNORE_COUNT: u32 = 1;

        let mut traces = if header {
            format!("Stack backtrace for task \"<{}>\" at line {} of \"{}\":\n",
                    thread::current().name().unwrap_or("unnamed"), self.line, self.file)
        } else {
            String::new()
        };

        let mut count = 0;

        if reverse {
            let mut symbols = Vec::new();

            for frame in self.backtrace.frames() {
                for symbol in frame.symbols() {
                    symbols.push(symbol);
                }
            }

            for symbol in symbols.iter().rev() {
                if count >= IGNORE_COUNT {
                    if let Some(name) = symbol.name() {
                        if let Some(name_str) = name.as_str() {
                            // Checks for `Backtrace::new` and `ThinBacktrace::new`
                            if name_str.contains("Backtrace::new") {
                                // Ignore and don't increment `count`
                                continue;
                            }
                        }
                    }

                    traces += Fmt::format_captured(count - IGNORE_COUNT, symbol).as_str();
                }

                count += 1;
            }
        } else {
            for frame in self.backtrace.frames() {
                for symbol in frame.symbols() {
                    if count >= IGNORE_COUNT {
                        if let Some(name) = symbol.name() {
                            if let Some(name_str) = name.as_str() {
                                // Checks for `Backtrace::new` AND `ThinBacktrace::new`
                                if name_str.contains("Backtrace::new") {
                                    // Ignore and don't increment `count`
                                    continue;
                                }
                            }
                        }

                        traces += Fmt::format_captured(count - IGNORE_COUNT, symbol).as_str();
                    }

                    count += 1;
                }
            }
        }

        traces
    }
}

impl From<Backtrace> for SourceBacktrace {
    fn from(backtrace: Backtrace) -> SourceBacktrace {
        SourceBacktrace { line: line!(), file: file!(), backtrace: backtrace }
    }
}

/// Returns a string containing the formatted backtrace and a header message
///
/// Pass a custom `BacktraceFmt` type to the macro to use custom formatting
#[macro_export]
macro_rules! backtrace {
    () => {
        backtrace!($crate::backtrace::DefaultBacktraceFmt)
    };

    ($fmt:ty) => {
        $crate::backtrace::format_trace::<$fmt>(true, line!(), file!())
    };
}

/// Variation of `backtrace!` that doesn't include the header line
#[macro_export]
macro_rules! backtrace_noheader {
    () => {
        backtrace_noheader!($crate::backtrace::DefaultBacktraceFmt)
    };

    ($fmt:ty) => {
        $crate::backtrace::format_trace::<$fmt>(false, line!(), file!())
    };
}