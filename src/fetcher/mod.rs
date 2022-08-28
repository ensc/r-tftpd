//

mod builder;
#[allow(clippy::module_inception)]
mod fetcher;
mod file;
mod memory;


pub use builder::Builder;
pub use fetcher::Fetcher;

use file::File;
use memory::Memory;

#[cfg(feature = "proxy")]
use r_tftpd_proxy::*;
#[cfg(feature = "proxy")]
pub use r_tftpd_proxy::{ Cache, CacheGcProperties };
