use std::{
    io::{Cursor, Error as IoError, ErrorKind},
    path::PathBuf,
};

use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend,
    sound::FromFileError,
    track::{TrackBuilder, TrackHandle},
};
use silica_asset::{AssetError, AssetSource, DirectorySource, load_bytes};

pub type BackendError = <DefaultBackend as kira::backend::Backend>::Error;
pub type AudioClip = kira::sound::static_sound::StaticSoundData;
pub type StreamingAudioClip = kira::sound::streaming::StreamingSoundData<FromFileError>;

fn map_io_error(error: FromFileError) -> IoError {
    if let FromFileError::IoError(error) = error {
        error
    } else {
        IoError::new(ErrorKind::InvalidData, error)
    }
}

pub struct Mixer {
    _manager: AudioManager<DefaultBackend>,
    music: TrackHandle,
    sfx: TrackHandle,
}

impl Mixer {
    pub fn new() -> Result<Self, BackendError> {
        let mut manager = AudioManager::new(AudioManagerSettings::default())?;
        let music = manager
            .add_sub_track(TrackBuilder::new().sub_track_capacity(0).sound_capacity(1))
            .expect("audio resource limit reached");
        let sfx = manager
            .add_sub_track(TrackBuilder::new().sub_track_capacity(0).sound_capacity(128))
            .expect("audio resource limit reached");
        Ok(Mixer {
            _manager: manager,
            music,
            sfx,
        })
    }
    pub fn is_music_playing(&self) -> bool {
        self.music.num_sounds() > 0
    }
    pub fn play_music(&mut self, clip: StreamingAudioClip) {
        match self.music.play(clip) {
            Ok(_) => (),
            Err(error) => log::error!("{}", error),
        }
    }
    pub fn play_sfx(&mut self, clip: AudioClip) {
        match self.sfx.play(clip) {
            Ok(_) => (),
            Err(error) => log::error!("{}", error),
        }
    }
}

pub fn load_audio_clip<S: AssetSource>(asset_source: &mut S, path: &str) -> Result<AudioClip, AssetError> {
    let data = load_bytes(asset_source, path)?;
    AudioClip::from_cursor(Cursor::new(data)).map_err(|e| AssetError::with_path(asset_source, path, map_io_error(e)))
}

pub struct Playlist {
    songs: Vec<PathBuf>,
    queue: Vec<PathBuf>,
    shuffle: bool,
}

impl Playlist {
    pub fn load(asset_source: &mut DirectorySource, path: &str, shuffle: bool) -> Result<Self, AssetError> {
        let base_path = asset_source.path();
        let songs = asset_source
            .read_directory(path)?
            .into_iter()
            .map(|file| base_path.join(file))
            .collect();
        Ok(Playlist {
            songs,
            queue: Vec::new(),
            shuffle,
        })
    }
    pub fn next_song(&mut self) -> Option<StreamingAudioClip> {
        if self.queue.is_empty() {
            self.queue = self.songs.clone();
            if self.shuffle {
                // TODO
            } else {
                self.queue.reverse();
            }
        }
        let path = self.queue.pop()?;
        match StreamingAudioClip::from_file(path) {
            Ok(audio_clip) => Some(audio_clip),
            Err(error) => {
                log::error!("{}", error);
                None
            }
        }
    }
    pub fn update_music(&mut self, mixer: &mut Mixer) {
        if !mixer.is_music_playing()
            && let Some(clip) = self.next_song()
        {
            mixer.play_music(clip);
        }
    }
}
