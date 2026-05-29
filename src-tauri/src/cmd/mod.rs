use anyhow::Result;
use smartstring::alias::String;

pub type CmdResult<T = ()> = Result<T, String>;

// Command modules
pub mod app;
pub mod backup;
pub mod clash;
pub mod lightweight;
pub mod media_unlock_checker;
pub mod network;
pub mod profile;
pub mod proxy;
pub mod runtime;
pub mod save_profile;
pub mod service;
pub mod system;
pub mod uwp;
pub mod validate;
pub mod verge;
pub mod webdav;

// Re-export all command functions for backwards compatibility
#[allow(unused_imports)]
pub use app::*;
#[allow(unused_imports)]
pub use backup::*;
#[allow(unused_imports)]
pub use clash::*;
#[allow(unused_imports)]
pub use lightweight::*;
pub use media_unlock_checker::*;
pub use network::*;
#[allow(unused_imports)]
pub use profile::*;
#[allow(unused_imports)]
pub use proxy::*;
pub use runtime::*;
#[allow(unused_imports)]
pub use save_profile::*;
#[allow(unused_imports)]
pub use service::*;
#[allow(unused_imports)]
pub use system::*;
#[allow(unused_imports)]
pub use uwp::*;
#[allow(unused_imports)]
pub use validate::*;
#[allow(unused_imports)]
pub use verge::*;
#[allow(unused_imports)]
pub use webdav::*;

pub trait StringifyErr<T> {
    fn stringify_err(self) -> CmdResult<T>;
    fn stringify_err_log<F>(self, log_fn: F) -> CmdResult<T>
    where
        F: Fn(&str);
}

impl<T, E: std::fmt::Display> StringifyErr<T> for Result<T, E> {
    fn stringify_err(self) -> CmdResult<T> {
        self.map_err(|e| e.to_string().into())
    }

    fn stringify_err_log<F>(self, log_fn: F) -> CmdResult<T>
    where
        F: Fn(&str),
    {
        self.map_err(|e| {
            let msg = String::from(e.to_string());
            log_fn(&msg);
            msg
        })
    }
}
