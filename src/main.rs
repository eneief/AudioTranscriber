mod audio;

use anyhow::Result;
use std::io::{self, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
mod deepgram_sdk;

fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let devices = audio::list_devices()?;

    println!("---------------------------------------------------");
    println!("Available input devices:\n");
    for (i, name) in devices.iter().enumerate() {
        println!("\tInput {i}: {name}");
    }

    print!("\nEnter device index (blank = default): ");
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let device_index = if line.trim().is_empty() {
        None
    } else {
        Some(line.trim().parse::<usize>().expect("invalid index"))
    };

    let mut rec = audio::Recorder::open(audio::RecorderConfig {
        device_index,
        prefer_mono: true,
        queue_chunks_capacity: 32,
    })?;

    println!(
        "\nSelected: {} \n\n\tinput_channels= {} | sample_rate= {} Hz | writing_channels= {}",
        rec.device_name(),
        rec.input_channels(),
        rec.sample_rate(),
        rec.output_channels(),
    );

    let mut sink = audio::WavSink::create("results/capture.wav", rec.sample_rate(), rec.output_channels())?;

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        // ctrlc::set_handler(move || stop.store(true, Ordering::SeqCst))?;
        ctrlc::set_handler(move || {
            eprintln!("\n\nStopping...");
            stop.store(true, Ordering::SeqCst);
        })?;
    }

    rec.start()?;
    eprintln!("\n\nRecording... \n\n\tpress Ctrl-C to stop.");
    // let mut total = 0usize;
    while !stop.load(Ordering::SeqCst) {
        if let Some(buf) = rec.recv_chunk_timeout(Duration::from_millis(100)) {
            // total += buf.len();
            sink.write_samples(&buf)?;
        }
    }

    rec.stop();
    while let Some(buf) = rec.try_recv_chunk() {
        // total += buf.len();
        sink.write_samples(&buf)?;
    }

    // eprintln!("Wrote {} samples (~{:.2} s)",
    //     total, total as f32 / (rec.sample_rate() as f32 * rec.output_channels() as f32));

    sink.finalize()?;
    eprintln!("Saved capture.wav");

    if let Ok(key) = std::env::var("DEEPGRAM_API_KEY") {
    eprintln!("Transcribing with Deepgram...\n\n");
    let rt = tokio::runtime::Runtime::new()?;
    match rt.block_on(deepgram_sdk::transcribe_file_sdk("results/capture.wav", &key)) {
        Ok(text) => {
            std::fs::write("results/transcript.txt", &text)?;
            println!("\nTranscript:\n{}\n", text);
            eprintln!("Saved transcript.txt");
        }
        Err(e) => eprintln!("Transcription failed: {e}"),
    }
} else {
    eprintln!("DEEPGRAM_API_KEY not set; skipping transcription.");
}

    Ok(())
}
