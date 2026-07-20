use loom::CompressionProfile;

pub enum Compression {
    None,
    Fast,
    Good,
    Best,
}

impl From<Compression> for CompressionProfile {
    fn from(profile: Compression) -> Self {
        match profile {
            Compression::None => CompressionProfile::Speed,
            Compression::Fast => CompressionProfile::Balanced,
            Compression::Good => CompressionProfile::Archive,
            Compression::Best => CompressionProfile::Brotli,
        }
    }
}
