# AudioTranscriber
Record audio from a selected audio input device using Rust and CPAL, write a WAV, then transcribe it with Deepgram's Nova-3 model using their Rust SDK. 

## How it works
### Audio Capture
- ```Recorder::open``` selects a device, asks for the device's default input. config, and then builds a CPAL input stream with format-specific callback.
- The callback converts to i16 and downmixed to mono.
- converted frames sent over a bounded channel to the main thread so that the audio isn't blocked by anything.
- in ```main.rs```, a while loop pulls chunks and writes them using WavSink.

### WAV output
- Wavsink::create in ```audio/wav.rs``` opens a writer with write_samples appending the sample and creates the .wav

### Transcription with Deepgram
- After the .wav is finalised, ```main.rs``` loads the deepgram api key, spins up a Tokio runtime, and calls transcribe_file_sdk using the .wav file and your deepgram key
- deepgram_sdk.rs opens the file asyncrhonously, constructs an AudioSource::from_buffer_with_mime_type() and then builds options for punctuation, filler words etc. and then calls dg.transcription().prerecorded()
- The best transcript is produced, printed in the terminal and saved to ```results/transcript.txt```

## Set up
### Prereqs
- Rust
- MacOS(I do not have a non MacOS device for which I could work with)

### clone + build
``` 
git clone <the repo url> 
cd AudioTranscriber
cargo build
```

### deepgram api key
create a .env in the repo's root.
``` 
cp .env.example .env
```
Then edit and paste your key.

### Run and use
``` 
cargo run
```

- You will see a list of input devices, select an index and press enter or just press enter to use default.
- Speak and then hit Ctrl+C to stop the recording.
- Review your audio and transcript in the results folder.


