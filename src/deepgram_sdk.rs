use anyhow::Result;
use deepgram::{
    common::{
        audio_source::AudioSource,
        options::{Language, Options, Model},
    },
    Deepgram,
};
use tokio::fs::File;

pub async fn transcribe_file_sdk(path: &str, api_key: &str) -> Result<String> {
    let dg = Deepgram::new(api_key)?;

    let file = File::open(path).await?;
    let source = AudioSource::from_buffer_with_mime_type(file, "audio/wav");

    let opts = Options::builder()
        .model(Model::CustomId(String::from("nova-3")))
        .punctuate(true)
        .smart_format(true)
        .language(Language::en_US)
        .diarize(true)
        .filler_words(true)
        .build();

    let resp = dg.transcription().prerecorded(source, &opts).await?;
    let text = resp
        .results
        .channels
        .get(0)
        .and_then(|c| c.alternatives.get(0))
        .map(|a| a.transcript.clone())
        .unwrap_or_default();

    Ok(text)
}
