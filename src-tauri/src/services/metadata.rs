use lofty::probe::Probe;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use std::path::Path;

#[derive(Debug, Default)]
pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub bit_rate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub channels: Option<i64>,
    pub picture_data: Option<Vec<u8>>,
    pub picture_mime: Option<String>,
}

pub fn extract_metadata<P: AsRef<Path>>(path: P) -> Result<AudioMetadata, String> {
    let mut metadata = AudioMetadata::default();
    
    let tagged_file = match Probe::open(path.as_ref()) {
        Ok(probe) => match probe.read() {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to read file: {}", e)),
        },
        Err(e) => return Err(format!("Failed to open file: {}", e)),
    };

    let properties = tagged_file.properties();
    metadata.duration_ms = Some(properties.duration().as_millis() as i64);
    metadata.bit_rate = properties.audio_bitrate().map(|b| b as i64 * 1000); // Usually kbps, depends on lofty version
    metadata.sample_rate = properties.sample_rate().map(|s| s as i64);
    metadata.channels = properties.channels().map(|c| c as i64);

    if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
        metadata.title = tag.title().map(|s| s.into_owned());
        metadata.artist = tag.artist().map(|s| s.into_owned());
        metadata.album = tag.album().map(|s| s.into_owned());
        
        if let Some(pic) = tag.pictures().first() {
            metadata.picture_data = Some(pic.data().to_vec());
            metadata.picture_mime = pic.mime_type().map(|m| m.to_string());
        }
    }

    Ok(metadata)
}
