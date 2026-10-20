//! Catálogo versionado, download retomável e selo de compatibilidade.

mod catalog;
mod download;
mod hub;
mod import;
mod seal;

pub use catalog::{BackendId, BackendNeed, Catalog, Kind, Model, ModelError, Source};
pub use download::{DownloadJob, fetch, prepare_download};
pub use hub::{HubFile, disk_name, find_file, hub_job, search, search_with};
pub use import::{WeightInfo, estimate_by_size, inspect_file};
pub use seal::{Compatibility, GpuVendor, MachineFacts, Reason, compatibility};
