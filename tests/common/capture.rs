//! Output capture utilities for tests.
#![allow(dead_code)]

use std::io::{Read, Write};

use gag::BufferRedirect;
use rich_rust::prelude::{Console, Segment};
use tracing::{debug, instrument, trace};

/// Captured stdout and stderr output.
#[derive(Debug, Default, Clone)]
pub struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
}

#[instrument(skip(func))]
pub fn capture_stdout<F: FnOnce()>(func: F) -> String {
    trace!("Capturing stdout");
    let mut redirect = BufferRedirect::stdout().expect("failed to redirect stdout");
    func();
    let _ = std::io::stdout().flush();
    let mut output = String::new();
    redirect
        .read_to_string(&mut output)
        .expect("failed to read stdout buffer");
    debug!(len = output.len(), "Captured stdout");
    output
}

#[instrument(skip(func))]
pub fn capture_stderr<F: FnOnce()>(func: F) -> String {
    trace!("Capturing stderr");
    let mut redirect = BufferRedirect::stderr().expect("failed to redirect stderr");
    func();
    let _ = std::io::stderr().flush();
    let mut output = String::new();
    redirect
        .read_to_string(&mut output)
        .expect("failed to read stderr buffer");
    debug!(len = output.len(), "Captured stderr");
    output
}

#[instrument(skip(func))]
pub fn capture_output<F: FnOnce()>(func: F) -> CapturedOutput {
    trace!("Capturing stdout+stderr");
    let mut stdout_redirect = BufferRedirect::stdout().expect("failed to redirect stdout");
    let mut stderr_redirect = BufferRedirect::stderr().expect("failed to redirect stderr");

    func();

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let mut stdout = String::new();
    let mut stderr = String::new();
    stdout_redirect
        .read_to_string(&mut stdout)
        .expect("failed to read stdout buffer");
    stderr_redirect
        .read_to_string(&mut stderr)
        .expect("failed to read stderr buffer");

    debug!(
        stdout_len = stdout.len(),
        stderr_len = stderr.len(),
        "Captured output"
    );
    CapturedOutput { stdout, stderr }
}

#[instrument(skip(console, func))]
pub fn capture_console_text<F: FnOnce()>(console: &mut Console, func: F) -> String {
    trace!("Capturing rich_rust Console output");
    console.begin_capture();
    func();
    let segments = console.end_capture();
    let text = segments_to_plain(&segments);
    debug!(len = text.len(), "Captured console text");
    text
}

fn segments_to_plain(segments: &[Segment<'_>]) -> String {
    segments.iter().map(|seg| seg.text.as_ref()).collect()
}
