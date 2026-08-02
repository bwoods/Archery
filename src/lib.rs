pub mod arrow;
mod entry;
pub mod extend;
pub mod file;
pub mod iter;
pub mod range;
pub mod transaction;

use loom::CompressionProfile;

#[derive(Copy, Clone)]
pub enum Compression {
    /// No secondary compression. Fastest encode/decode.
    None,
    /// [LZ4] post-pass on encoded blocks. Fast decode, ~2–3× extra compression.
    ///
    /// [LZ4]: https://lz4.org
    Fast,
    /// [Zstd] post-pass on encoded blocks. Good ratio, higher CPU cost.
    ///
    /// [Zstd]: https://facebook.github.io/zstd/
    Good,
    /// [Brotli] post-pass on string/binary blocks, [Zstd] on numeric blocks.
    /// Best compression ratio for text-heavy workloads.
    ///
    /// [brotli]: https://brotli.org
    /// [Zstd]: https://facebook.github.io/zstd/
    Best,
}

impl From<Compression> for CompressionProfile {
    fn from(compression: Compression) -> Self {
        match compression {
            Compression::None => CompressionProfile::Speed,
            Compression::Fast => CompressionProfile::Balanced,
            Compression::Good => CompressionProfile::Archive,
            Compression::Best => CompressionProfile::Brotli,
        }
    }
}
