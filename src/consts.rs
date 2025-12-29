pub(crate) const THINK_START: &str = "<think>";
pub(crate) const THINK_END: &str = "</think>";

pub(crate) const REASONING_CUTOFF_STUB: &str =
    "Right, this is taking too long... Time to write the answer.";

pub(crate) const DEFAULT_MAX_TOKENS: i32 = 1024 * 1024;

#[allow(dead_code)]
pub(crate) const CONNECT_TIMEOUT_SECS: u64 = 30;
#[allow(dead_code)]
pub(crate) const READ_TIMEOUT_SECS: u64 = 60;
pub const CHANNEL_BUFFER_SIZE: usize = 100;
#[allow(dead_code)]
pub(crate) const SERVER_PORT: u16 = 8080;
