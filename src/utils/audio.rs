use rodio::{
    Decoder, OutputStream, Sink, Source,
    source::{SineWave},
};
use std::{
    fs::File,
    io::{BufReader, Error as IoError},
    path::{Path, PathBuf},
    time::Duration,
    sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}},
    collections::{HashMap, VecDeque},
};
use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;
use thiserror::Error;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

// 1. Error Types ===============================================

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Invalid audio format")]
    InvalidFormat,
    #[error("Audio device error")]
    DeviceError,
    #[error("Playback error: {0}")]
    PlaybackError(String),
    #[error("IO error: {0}")]
    IoError(#[from] IoError),
    #[error("Sound not found")]
    SoundNotFound,
    #[error("Audio system not initialized")]
    NotInitialized,
}

// 2. Settings ==================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub voice_volume: f32,
    pub music_fade_duration: f32,
    pub sample_rate: u32,
    pub music_playlist: Vec<PathBuf>,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 0.7,
            sfx_volume: 0.8,
            voice_volume: 0.9,
            music_fade_duration: 1.5,
            sample_rate: 44100,
            music_playlist: Vec::new(),
        }
    }
}

// 3. Audio Handle ==============================================

#[derive(Debug, Clone)]
pub struct AudioHandle {
    id: usize,
    sink: Arc<Mutex<Option<Sink>>>,
    volume: Arc<AtomicUsize>, // Stored as u16 fixed-point (0.0-1.0)
}

impl AudioHandle {
    pub fn set_volume(&self, volume: f32) {
        let fixed = (volume.clamp(0.0, 1.0) * 65535.0) as usize;
        self.volume.store(fixed, Ordering::Relaxed);
        
        if let Ok(guard) = self.sink.lock() {
            if let Some(sink) = guard.as_ref() {
                sink.set_volume(volume);
            }
        }
    }

    pub fn stop(self) {
        if let Ok(mut guard) = self.sink.lock() {
            if let Some(sink) = guard.take() {
                sink.stop();
            }
        }
    }

    pub fn detach(self) {
        std::mem::forget(self);
    }
}

impl Drop for AudioHandle {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.sink.lock() {
            if let Some(sink) = guard.take() {
                sink.stop();
            }
        }
    }
}

// 4. Music Settings ============================================

#[derive(Debug, Clone)]
pub struct MusicSettings {
    pub volume: f32,
    pub looping: bool,
    pub start_time: Option<Duration>,
}

// 5. Main Audio Player =========================================

pub struct AudioPlayer {
    _stream: OutputStream, // Kept to prevent the stream from being dropped
    stream_handle: rodio::OutputStreamHandle,
    music_sink: Option<Sink>,
    settings: RwLock<AudioSettings>,
    sfx_handles: RwLock<HashMap<usize, AudioHandle>>,
    next_sfx_id: AtomicUsize,
    music_queue: RwLock<VecDeque<(PathBuf, MusicSettings)>>,
    current_music: RwLock<Option<(PathBuf, MusicSettings)>>,
}

impl AudioPlayer {
    // Initialization --------------------------------------------
    pub fn new(settings: AudioSettings) -> Result<Self, AudioError> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|_| AudioError::DeviceError)?;
        
        Ok(Self {
            _stream: stream,
            stream_handle,
            music_sink: None,
            settings: RwLock::new(settings),
            sfx_handles: RwLock::new(HashMap::new()),
            next_sfx_id: AtomicUsize::new(0),
            music_queue: RwLock::new(VecDeque::new()),
            current_music: RwLock::new(None),
        })
    }

    // Music Control ---------------------------------------------
    pub fn play_music(
        &mut self,
        path: &Path,
        settings: MusicSettings,
    ) -> Result<(), AudioError> {
        // Stop any playing music
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
        
        // Create a new sink
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|_| AudioError::DeviceError)?;
            
        // Set the volume
        let current_settings = self.settings.read();
        sink.set_volume(current_settings.master_volume * current_settings.music_volume * settings.volume);
        
        // Load and play the source
        let file = File::open(path).map_err(|_| AudioError::FileNotFound(path.to_path_buf()))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|_| AudioError::InvalidFormat)?;
            
        // Apply settings
        let mut processed_source = source.convert_samples::<f32>();
        
        if let Some(start_time) = settings.start_time {
            processed_source = processed_source.skip_duration(start_time);
        }
        
        if settings.looping {
            processed_source = processed_source.repeat_infinite();
        }
        
        sink.append(processed_source);
        self.music_sink = Some(sink);
        
        // Update current music
        *self.current_music.write() = Some((path.to_path_buf(), settings));
        
        Ok(())
    }

    pub fn queue_music(&self, path: &Path, settings: MusicSettings) -> Result<(), AudioError> {
        self.music_queue.write().push_back((path.to_path_buf(), settings));
        Ok(())
    }

    pub fn skip_to_next(&mut self) -> Result<(), AudioError> {
        let mut queue = self.music_queue.write();
        if let Some((path, settings)) = queue.pop_front() {
            self.play_music(&path, settings)
        } else {
            Err(AudioError::SoundNotFound)
        }
    }

    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
        *self.current_music.write() = None;
    }

    pub fn pause_music(&self) {
        if let Some(sink) = &self.music_sink {
            sink.pause();
        }
    }

    pub fn resume_music(&self) {
        if let Some(sink) = &self.music_sink {
            sink.play();
        }
    }

    pub fn is_music_playing(&self) -> bool {
        if let Some(sink) = &self.music_sink {
            !sink.empty() && !sink.is_paused()
        } else {
            false
        }
    }

    // SFX Control -----------------------------------------------
    pub fn play_sfx(
        &self,
        path: &Path,
        volume: f32,
    ) -> Result<AudioHandle, AudioError> {
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|_| AudioError::DeviceError)?;
            
        let file = File::open(path).map_err(|_| AudioError::FileNotFound(path.to_path_buf()))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|_| AudioError::InvalidFormat)?;
            
        // Apply settings
        let settings = self.settings.read();
        let final_volume = settings.master_volume * settings.sfx_volume * volume;
        sink.set_volume(final_volume);
            
        sink.append(source.convert_samples());
        
        let id = self.next_sfx_id.fetch_add(1, Ordering::Relaxed);
        let sink_arc = Arc::new(Mutex::new(Some(sink)));
        let volume_arc = Arc::new(AtomicUsize::new((final_volume * 65535.0) as usize));
        
        let handle = AudioHandle {
            id,
            sink: sink_arc,
            volume: volume_arc,
        };
        
        self.sfx_handles.write().insert(id, handle.clone());
        
        Ok(handle)
    }

    // Voice Control ---------------------------------------------
    pub fn play_voice(&self, path: &Path) -> Result<AudioHandle, AudioError> {
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|_| AudioError::DeviceError)?;
            
        let file = File::open(path).map_err(|_| AudioError::FileNotFound(path.to_path_buf()))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|_| AudioError::InvalidFormat)?;
            
        // Apply settings
        let settings = self.settings.read();
        let final_volume = settings.master_volume * settings.voice_volume;
        sink.set_volume(final_volume);
            
        sink.append(source.convert_samples());
        
        let id = self.next_sfx_id.fetch_add(1, Ordering::Relaxed);
        let sink_arc = Arc::new(Mutex::new(Some(sink)));
        let volume_arc = Arc::new(AtomicUsize::new((final_volume * 65535.0) as usize));
        
        let handle = AudioHandle {
            id,
            sink: sink_arc,
            volume: volume_arc,
        };
        
        self.sfx_handles.write().insert(id, handle.clone());
        
        Ok(handle)
    }

    // Volume Control --------------------------------------------
    pub fn set_master_volume(&self, volume: f32) {
        self.settings.write().master_volume = volume.clamp(0.0, 1.0);
        self.update_volumes();
    }

    pub fn set_music_volume(&self, volume: f32) {
        self.settings.write().music_volume = volume.clamp(0.0, 1.0);
        self.update_volumes();
    }

    pub fn set_sfx_volume(&self, volume: f32) {
        self.settings.write().sfx_volume = volume.clamp(0.0, 1.0);
        self.update_volumes();
    }

    pub fn set_voice_volume(&self, volume: f32) {
        self.settings.write().voice_volume = volume.clamp(0.0, 1.0);
        self.update_volumes();
    }

    fn update_volumes(&self) {
        let settings = self.settings.read();
        
        // Update music volume
        if let Some(sink) = &self.music_sink {
            if let Some((_, music_settings)) = &*self.current_music.read() {
                let volume = settings.master_volume * settings.music_volume * music_settings.volume;
                sink.set_volume(volume);
            }
        }
        
        // We don't update SFX volumes here as those are one-shot sounds
        // Volume changes will apply to new sounds
    }
    
    // Utility Methods -------------------------------------------
    pub fn current_music(&self) -> Option<(PathBuf, MusicSettings)> {
        self.current_music.read().clone()
    }

    pub fn music_queue(&self) -> Vec<(PathBuf, MusicSettings)> {
        self.music_queue.read().iter().cloned().collect()
    }

    pub fn clear_music_queue(&self) {
        self.music_queue.write().clear();
    }
    
    // Cleanup Methods -------------------------------------------
    pub fn cleanup_finished_sfx(&self) {
        let mut handles = self.sfx_handles.write();
        let mut to_remove = Vec::new();
        
        for (id, handle) in handles.iter() {
            if let Ok(guard) = handle.sink.lock() {
                if let Some(sink) = guard.as_ref() {
                    if sink.empty() {
                        to_remove.push(*id);
                    }
                } else {
                    to_remove.push(*id);
                }
            }
        }
        
        for id in to_remove {
            handles.remove(&id);
        }
    }
}

// 6. Global Audio System =======================================

lazy_static! {
    static ref AUDIO_SYSTEM: RwLock<Option<AudioPlayer>> = RwLock::new(None);
}

pub fn init_audio(settings: AudioSettings) -> Result<(), AudioError> {
    let mut system = AUDIO_SYSTEM.write();
    if system.is_some() {
        return Ok(()); // Already initialized
    }
    *system = Some(AudioPlayer::new(settings)?);
    Ok(())
}

pub fn get_audio() -> Result<RwLockReadGuard<'static, Option<AudioPlayer>>, AudioError> {
    Ok(AUDIO_SYSTEM.read())
}

pub fn get_audio_mut() -> Result<RwLockWriteGuard<'static, Option<AudioPlayer>>, AudioError> {
    Ok(AUDIO_SYSTEM.write())
}

// 7. Utility Functions =========================================

pub fn create_music_settings(
    volume: f32,
    looping: bool,
    start_time: Option<Duration>,
) -> MusicSettings {
    MusicSettings {
        volume: volume.clamp(0.0, 1.0),
        looping,
        start_time,
    }
}

// 8. Test Sound ================================================

pub fn play_test_sound() -> Result<(), AudioError> {
    let audio = get_audio()?;
    
    if let Some(audio) = audio.as_ref() {
        let sink = Sink::try_new(&audio.stream_handle)
            .map_err(|_| AudioError::DeviceError)?;
        
        let source = SineWave::new(440.0)
            .take_duration(Duration::from_secs_f32(0.5))
            .amplify(0.2);
        
        sink.append(source);
        Ok(())
    } else {
        Err(AudioError::NotInitialized)
    }
    }
