//! FFmpeg/FFprobe integration: toolchain detection (section 19/21),
//! binary resolution (section 20), structured probing (section 18) and
//! thumbnail extraction (section 24). No transcoding happens anywhere in
//! this module.

pub mod probe;
pub mod resolver;
pub mod thumbnail;
pub mod toolchain;

pub use probe::FfprobeMediaProbeService;
pub use thumbnail::FfmpegThumbnailService;
pub use toolchain::FfmpegMediaService;
