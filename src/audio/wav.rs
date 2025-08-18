use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{fs::File, io::BufWriter, path::Path};

pub struct WavSink {
    writer: WavWriter<BufWriter<File>>,
}

impl WavSink {
    pub fn create<P: AsRef<Path>>(path: P, sample_rate_hz: u32, channels: u16) -> Result<Self> {
        let spec = WavSpec {
            channels,
            sample_rate: sample_rate_hz,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let writer = WavWriter::create(path, spec)?;
        Ok(Self { writer })
    }

    pub fn write_samples(&mut self, data: &[i16]) -> Result<()> {
        for &s in data {
            self.writer.write_sample(s)?;
        }
        Ok(())
    }

    pub fn finalize(self) -> Result<()> {
        self.writer.finalize()?;
        Ok(())
    }
}
