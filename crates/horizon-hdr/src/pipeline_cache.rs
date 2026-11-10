//! Halcyon: the driver's compiled pipelines, kept in a file between runs.
//!
//! Without a cache every start compiles every shader pipeline again, and a phone's driver takes seconds to
//! do it (the long frames while the title menu and the world first draw). The cache is the driver's own
//! binary, so it lives here: creating one from stored bytes is an unsafe call the game crate forbids.
//!
//! It is on only when `HORIZON_PIPELINE_CACHE` names a file (the Android host sets it) and the device has
//! the feature (Vulkan). The file is written again while the game runs, whenever the cache has grown.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

static CACHE: OnceLock<wgpu::PipelineCache> = OnceLock::new();

/// How often the cache's size is looked at.
const SAVE_EVERY: Duration = Duration::from_secs(15);

/// Opens the cache from its file and starts keeping the file up to date. Call it once, with the new device.
pub fn init_pipeline_cache(device: &wgpu::Device) {
    let Some(path) = std::env::var_os("HORIZON_PIPELINE_CACHE").map(PathBuf::from) else {
        return;
    };
    if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
        log::info!("Horizon pipeline cache: the device has no pipeline cache");
        return;
    }
    let stored = std::fs::read(&path).ok().filter(|bytes| !bytes.is_empty());
    log::info!(
        "Horizon pipeline cache: {} ({})",
        path.display(),
        stored
            .as_ref()
            .map_or_else(|| "none stored yet".to_owned(), |b| format!("{} bytes stored", b.len()))
    );
    // SAFETY: the bytes are only ever what `get_data` returned on this machine (the file is ours); the
    // driver checks their header, and with `fallback` a cache it does not accept starts empty instead.
    let cache = unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("Horizon pipelines"),
            data: stored.as_deref(),
            fallback: true,
        })
    };
    if CACHE.set(cache).is_err() {
        return;
    }
    let mut saved = stored.map_or(0, |bytes| bytes.len());
    let spawned = std::thread::Builder::new()
        .name("pipeline-cache".to_owned())
        .spawn(move || {
            loop {
                std::thread::sleep(SAVE_EVERY);
                let Some(bytes) = CACHE.get().and_then(|cache| cache.get_data()) else {
                    continue;
                };
                if bytes.is_empty() || bytes.len() == saved {
                    continue;
                }
                // A file written whole and then renamed: a game killed in the middle leaves the old one.
                let temporary = path.with_extension("tmp");
                if std::fs::write(&temporary, &bytes).is_ok() && std::fs::rename(&temporary, &path).is_ok() {
                    log::info!("Horizon pipeline cache: saved {} bytes", bytes.len());
                    saved = bytes.len();
                }
            }
        });
    if let Err(e) = spawned {
        log::warn!("Horizon pipeline cache: the saving thread did not start: {e}");
    }
}

/// The cache to give a pipeline's descriptor, once [`init_pipeline_cache`] made one.
pub fn pipeline_cache() -> Option<&'static wgpu::PipelineCache> {
    CACHE.get()
}
