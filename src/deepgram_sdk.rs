use anyhow::Result;
use deepgram::{
    Deepgram,
    common::{
        audio_source::AudioSource,
        options::{Language, Model, Options},
    },
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
        .first()
        .and_then(|c| c.alternatives.first())
        .map(|a| a.transcript.clone())
        .unwrap_or_default();

    Ok(text)
}
