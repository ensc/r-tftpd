#![allow(clippy::redundant_field_names)]

#[macro_use]
extern crate tracing;

pub mod errors;
pub use errors::{ Error, Result };

mod uri;
pub use uri::Uri;

mod cache;
pub use cache::{ Cache,
		 GcProperties as CacheGcProperties,
		 Entry as CacheEntry,
		 LookupResult as CacheLookupResult,
		 EntryData as CacheEntryData };

mod http;
mod util;
