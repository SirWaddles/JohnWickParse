use crate::assets::{USoundWave, ParserResult, ParserError};

pub fn decode_sound(sound: USoundWave) -> ParserResult<Vec<u8>> {
    let mut sound_data = Vec::new();
    if sound.is_streaming() {
        if sound.get_stream_format() != "OGG" {
            return Err(ParserError::new(format!("Format Unsupported: {}", sound.get_stream_format())));
        }
        let chunks = sound.get_stream_chunks();
        for chunk in chunks {
            let data_size = chunk.get_audio_size();
            let mut data = chunk.get_audio_data();
            data.truncate(data_size as usize);
            sound_data.append(&mut data);
        }
    } else {
        let chunks = sound.get_audio_chunks();
        for chunk in chunks {
            if !chunk.get_format().contains("OGG") {
                return Err(ParserError::new(format!("Format Unsupported: {}", chunk.get_format())));
            }
            sound_data.append(&mut chunk.get_data());
        }
    }
    Ok(sound_data)
}