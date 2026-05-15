use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WavInfo {
    pub duration_sec: f64,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn probe_wav(path: &str) -> Result<WavInfo, String> {
    let reader = hound::WavReader::open(path).map_err(|error| error.to_string())?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels;
    let duration_samples = reader.duration() as f64;
    let duration_sec = duration_samples / sample_rate as f64 / channels as f64;

    Ok(WavInfo {
        duration_sec,
        sample_rate,
        channels,
    })
}
