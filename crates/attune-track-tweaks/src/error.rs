use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrackTweaksError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Audio decode error: {0}")]
    Decode(String),

    #[error("No supported audio track found in file")]
    NoSupportedTrack,

    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    #[error("Metadata parse/write error: {0}")]
    Metadata(String),
}
