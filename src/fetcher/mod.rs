//

mod builder;
#[allow(clippy::module_inception)]
mod fetcher;
mod file;
mod memory;


pub use builder::Builder;

use file::File;
use memory::Memory;

#[cfg(feature = "proxy")]
mod uri;
#[cfg(feature = "proxy")]
use uri::Uri;

#[cfg(feature = "proxy")]
mod cache;
#[cfg(feature = "proxy")]
pub use cache::{ Cache,
		 Entry as CacheEntry,
		 LookupResult as CacheLookupResult,
		 EntryData as CacheEntryData };

#[cfg(feature = "proxy")]
mod http;

pub use fetcher::Fetcher;
