use super::*;

#[derive(Debug, Serialize)]
pub struct FStreamedAudioChunk {
    data: FByteBulkData,
    data_size: i32,
    audio_size: i32,
}

impl FStreamedAudioChunk {
    fn new(reader: &mut ReaderCursor, ubulk: &mut Option<ReaderCursor>, bulk_offset: i64) -> ParserResult<Self> {
        let _cooked = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            data: FByteBulkData::new(reader, ubulk, bulk_offset)?,
            data_size: reader.read_i32::<LittleEndian>()?,
            audio_size: reader.read_i32::<LittleEndian>()?,
        })
    }

    pub fn get_audio_size(&self) -> i32 {
        self.audio_size
    }

    pub fn get_audio_data(self) -> Vec<u8> {
        self.data.data
    }
}

#[derive(Debug, Serialize)]
pub struct FFormatContainer {
    name: String,
    data: FByteBulkData,
}

impl FFormatContainer {
    fn new(reader: &mut ReaderCursor, name_map: &NameMap, ubulk: &mut Option<ReaderCursor>, bulk_offset: i64) -> ParserResult<Self> {
        Ok(Self {
            name: read_fname(reader, name_map)?,
            data: FByteBulkData::new(reader, ubulk, bulk_offset)?,
        })
    }

    pub fn get_format(&self) -> &str {
        &self.name
    }

    pub fn get_data(self) -> Vec<u8> {
        self.data.data
    }
}

#[derive(Debug, Serialize)]
pub struct USoundWave {
    super_object: UObject,
    guid: FGuid,
    streaming: bool,
    audio_data: Vec<FFormatContainer>,
    streamed_audio: Vec<FStreamedAudioChunk>,
    stream_format: String,
}

impl PackageExport for USoundWave {
    fn get_export_type(&self) -> &str {
        "SoundWave"
    }
}

impl USoundWave {
    pub(super) fn new(reader: &mut ReaderCursor, name_map: &NameMap, import_map: &ImportMap, asset_file_size: i32, export_size: i64, ubulk: &mut Option<ReaderCursor>) -> ParserResult<Self> {
        let super_object = UObject::new(reader, name_map, import_map, "SoundWave")?;
        let streaming = match super_object.get_boolean("bStreaming") {
            Some(data) => data,
            None => false,
        };

        let _cooked = reader.read_u32::<LittleEndian>()? != 0;
        let mut audio_data = Vec::new();
        let bulk_offset = export_size + asset_file_size as i64;

        if !streaming {
            let num_elements = reader.read_u32::<LittleEndian>()?;
            for _i in 0..num_elements {
                audio_data.push(FFormatContainer::new(reader, name_map, ubulk, bulk_offset)?);
            }
        }

        let guid = FGuid::new(reader)?;
        let mut streamed_audio = Vec::new();
        let mut stream_format = "".to_owned();

        if streaming {
            let num_chunks = reader.read_u32::<LittleEndian>()?;
            stream_format = read_fname(reader, name_map)?;
            for _i in 0..num_chunks {
                let chunk = FStreamedAudioChunk::new(reader, ubulk, bulk_offset)?;
                streamed_audio.push(chunk);
            }
        }


        Ok(Self {
            super_object, guid, streaming, audio_data, streamed_audio, stream_format,
        })
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    pub fn get_stream_chunks(self) -> Vec<FStreamedAudioChunk> {
        self.streamed_audio
    }

    pub fn get_audio_chunks(self) -> Vec<FFormatContainer> {
        self.audio_data
    }

    pub fn get_stream_format(&self) -> &str {
        &self.stream_format
    }
}