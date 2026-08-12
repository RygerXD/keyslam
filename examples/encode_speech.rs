use std::{env, fs, io::BufReader, path::Path};

use rodio::Source;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = env::args().nth(1).ok_or("missing WAV directory")?;
    let verify = env::args().nth(2).as_deref() == Some("--verify");
    process_directory(Path::new(&root), verify)
}

fn process_directory(directory: &Path, verify: bool) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            process_directory(&path, verify)?;
        } else if path.extension().is_some_and(|extension| extension == "wav") {
            let decoder = rodio::Decoder::try_from(BufReader::new(fs::File::open(&path)?))?;
            let channels = usize::from(decoder.channels().get());
            let source_rate = decoder.sample_rate().get() as f64;
            let source = decoder.collect::<Vec<_>>();
            let mono = source
                .chunks(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                .collect::<Vec<_>>();
            let frames = (mono.len() as f64 * 48_000.0 / source_rate).ceil() as usize;
            let mut resampled = Vec::with_capacity(frames);
            for frame in 0..frames {
                let position = frame as f64 * source_rate / 48_000.0;
                let index = position.floor() as usize;
                let next = (index + 1).min(mono.len().saturating_sub(1));
                let fraction = (position - index as f64) as f32;
                resampled.push(mono[index] + (mono[next] - mono[index]) * fraction);
            }
            let opus = ruopus::encode_ogg_opus(&resampled, 1, 32_000);
            fs::write(path.with_extension("opus"), opus)?;
        } else if verify
            && path
                .extension()
                .is_some_and(|extension| extension == "opus")
        {
            let bytes = fs::read(&path)?;
            let (samples, head) = ruopus::decode_ogg_opus(&bytes)?;
            if head.channel_count == 0 || samples.is_empty() {
                return Err(format!("{} contains no audio", path.display()).into());
            }
        }
    }
    Ok(())
}
