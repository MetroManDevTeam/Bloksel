use rodio::{
    Decoder, OutputStream, Sink, Source, 
    dynamic_mixer::{DynamicMixerController, mixer},
    source::{Buffered, SamplesConverter, TakeDuration, Repeat},
    Queue, OutputStreamHandle, source::UniformSourceIterator,
    Sample, PlayError, source::SineWave,
};
use std::{
    fs::File,
    io::{BufReader, Error as IoError},
    path::{Path, PathBuf},
    time::Duration,
    sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}},
    collections::{HashMap, VecDeque},
    fmt,
    iter,
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
    #[error("Channel limit reached")]
    ChannelLimit,
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
    pub enable_reverb: bool,
    pub enable_spatial: bool,
    pub max_sfx_channels: usize,
    pub music_fade_duration: f32,
    pub low_pass_filter: bool,
    pub high_pass_filter: bool,
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
            enable_reverb: true,
            enable_spatial: false,
            max_sfx_channels: 32,
            music_fade_duration: 1.5,
            low_pass_filter: false,
            high_pass_filter: false,
            sample_rate: 44100,
            music_playlist: Vec::new(),
        }
    }
}

// 3. Audio Handle ==============================================

#[derive(Debug, Clone)]
pub struct AudioHandle {
    id: usize,
    controller: Arc<Mutex<DynamicMixerController<f32>>>,
    volume: Arc<AtomicUsize>, // Stored as u16 fixed-point (0.0-1.0)
}

impl AudioHandle {
    pub fn set_volume(&self, volume: f32) {
        let fixed = (volume.clamp(0.0, 1.0) * 65535.0) as usize;
        self.volume.store(fixed, Ordering::Relaxed);
    }

    pub fn stop(self) {
        // Handled by Drop
    }

    pub fn detach(self) {
        std::mem::forget(self);
    }
}

impl Drop for AudioHandle {
    fn drop(&mut self) {
        if let Ok(mut controller) = self.controller.lock() {
            controller.remove(self.id);
        }
    }
}

// 4. Fade State ================================================

#[derive(Debug, Clone)]
enum FadeState {
    Idle,
    FadingIn {
        target_volume: f32,
        duration: Duration,
        elapsed: Duration,
    },
    FadingOut {
        duration: Duration,
        elapsed: Duration,
    },
    Crossfading {
        out_duration: Duration,
        in_duration: Duration,
        out_elapsed: Duration,
        next_track: PathBuf,
        next_settings: MusicSettings,
    },
}

#[derive(Debug, Clone)]
struct MusicSettings {
    volume: f32,
    looping: bool,
    start_time: Option<Duration>,
}

// 5. Main Audio Player =========================================

pub struct AudioPlayer {
    _stream: OutputStream, // Kept to prevent the stream from being dropped
    stream_handle: OutputStreamHandle,
    music_sink: Sink,
    sfx_controller: Arc<Mutex<DynamicMixerController<f32>>>,
    voice_sink: Sink,
    settings: RwLock<AudioSettings>,
    fade_state: RwLock<FadeState>,
    current_music_volume: AtomicUsize,
    music_queue: RwLock<VecDeque<(PathBuf, MusicSettings)>>,
    current_music: RwLock<Option<(PathBuf, MusicSettings)>>,
    sfx_cache: RwLock<HashMap<PathBuf, Buffered<SamplesConverter<Decoder<BufReader<File>>, f32>>>>,
    next_sfx_id: AtomicUsize,
    reverb_send: f32,
    low_pass_cutoff: f32,
    high_pass_cutoff: f32,
}

impl AudioPlayer {
    // Initialization --------------------------------------------
    pub fn new(settings: AudioSettings) -> Result<Self, AudioError> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|_| AudioError::DeviceError)?;
        
        let music_sink = Sink::try_new(&stream_handle)
            .map_err(|_| AudioError::DeviceError)?;
        
        let voice_sink = Sink::try_new(&stream_handle)
            .map_err(|_| AudioError::DeviceError)?;
        
        let (sfx_controller, sfx_mixer) = mixer(1, settings.sample_rate);
        stream_handle.play_raw(sfx_mixer)
            .map_err(|_| AudioError::DeviceError)?;
        
        let sfx_controller = Arc::new(Mutex::new(sfx_controller));
        
        Ok(Self {
            _stream: stream,
            stream_handle,
            music_sink,
            sfx_controller,
            voice_sink,
            settings: RwLock::new(settings),
            fade_state: RwLock::new(FadeState::Idle),
            current_music_volume: AtomicUsize::new((0.7 * 65535.0) as usize),
            music_queue: RwLock::new(VecDeque::new()),
            current_music: RwLock::new(None),
            sfx_cache: RwLock::new(HashMap::new()),
            next_sfx_id: AtomicUsize::new(0),
            reverb_send: 0.0,
            low_pass_cutoff: 22000.0,
            high_pass_cutoff: 20.0,
        })
    }

    // Music Control ---------------------------------------------
    pub fn play_music(
        &self,
        path: &Path,
        settings: MusicSettings,
        fade_in: bool,
    ) -> Result<(), AudioError> {
        let duration = if fade_in {
            Duration::from_secs_f32(self.settings.read().music_fade_duration)
        } else {
            Duration::ZERO
        };
        
        self._play_music(path, settings, duration)
    }
    
    fn _play_music(
        &self,
        path: &Path,
        settings: MusicSettings,
        fade_in: Duration,
    ) -> Result<(), AudioError> {
        let mut source = self.load_audio_file(path)?;
        
        // Apply settings
        if let Some(start_time) = settings.start_time {
            source = source.skip_duration(start_time);
        }
        if settings.looping {
            source = source.repeat_infinite();
        }
        
        let mut music_sink = Sink::try_new(&self.stream_handle)
            .map_err(|_| AudioError::DeviceError)?;
        
        if fade_in > Duration::ZERO {
            music_sink.set_volume(0.0);
            *self.fade_state.write() = FadeState::FadingIn {
                target_volume: settings.volume,
                duration: fade_in,
                elapsed: Duration::ZERO,
            };
        } else {
            music_sink.set_volume(settings.volume);
        }
        
        music_sink.append(source);
        *self.current_music.write() = Some((path.to_path_buf(), settings));
        Ok(())
    }
    
    pub fn queue_music(&self, path: &Path, settings: MusicSettings) -> Result<(), AudioError> {
        self.music_queue.write().push_back((path.to_path_buf(), settings));
        Ok(())
    }

    pub fn skip_to_next(&self) -> Result<(), AudioError> {
        let mut queue = self.music_queue.write();
        if let Some((path, settings)) = queue.pop_front() {
            self._play_music(&path, settings, Duration::from_secs_f32(self.settings.read().music_fade_duration))
        } else {
            Err(AudioError::SoundNotFound)
        }
    }

    pub fn fade_out_music(&self, duration: Duration) {
        *self.fade_state.write() = FadeState::FadingOut {
            duration,
            elapsed: Duration::ZERO,
        };
    }

    pub fn crossfade_to(
        &self,
        path: &Path,
        settings: MusicSettings,
        out_duration: Duration,
        in_duration: Duration,
    ) -> Result<(), AudioError> {
        *self.fade_state.write() = FadeState::Crossfading {
            out_duration,
            in_duration,
            out_elapsed: Duration::ZERO,
            next_track: path.to_path_buf(),
            next_settings: settings,
        };
        Ok(())
    }

    pub fn stop_music(&self) {
        self.music_sink.stop();
        *self.fade_state.write() = FadeState::Idle;
        *self.current_music.write() = None;
    }

    pub fn pause_music(&self) {
        self.music_sink.pause();
    }

    pub fn resume_music(&self) {
        self.music_sink.play();
    }

    pub fn is_music_playing(&self) -> bool {
        !self.music_sink.empty() && !self.music_sink.is_paused()
    }

    // SFX Control -----------------------------------------------
    pub fn play_sfx(
        &self,
        path: &Path,
        volume: f32,
        spatial: Option<AudioPosition>,
    ) -> Result<AudioHandle, AudioError> {
        let settings = self.settings.read();
        if self.next_sfx_id.load(Ordering::Relaxed) >= settings.max_sfx_channels {
            return Err(AudioError::ChannelLimit);
        }

        let mut source = self.load_audio_file(path)?;
        
        // Apply spatial audio if enabled
        let final_volume = if settings.enable_spatial {
            if let Some(spatial) = spatial {
                let listener = AudioPosition::default(); // Default listener at origin
                volume * calculate_spatial_volume(listener, spatial)
            } else {
                volume
            }
        } else {
            volume
        };

        let id = self.next_sfx_id.fetch_add(1, Ordering::Relaxed);
        let volume_arc = Arc::new(AtomicUsize::new((final_volume * 65535.0) as usize));
        
        let controller = self.sfx_controller.clone();
        controller.lock().unwrap().add(source.amplify(final_volume), id);
        
        Ok(AudioHandle {
            id,
            controller,
            volume: volume_arc,
        })
    }

    pub fn preload_sfx(&self, path: &Path) -> Result<(), AudioError> {
        if self.sfx_cache.read().contains_key(path) {
            return Ok(());
        }

        let file = File::open(path).map_err(|_| AudioError::FileNotFound(path.to_path_buf()))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|_| AudioError::InvalidFormat)?;
        
        let converted = source.convert_samples::<f32>();
        let buffered = converted.buffered();
        
        self.sfx_cache.write().insert(path.to_path_buf(), buffered);
        Ok(())
    }

    pub fn unload_sfx(&self, path: &Path) {
        self.sfx_cache.write().remove(path);
    }

    // Voice Control ---------------------------------------------
    pub fn play_voice(&self, path: &Path) -> Result<(), AudioError> {
        let source = self.load_audio_file(path)?;
        self.voice_sink.append(source);
        Ok(())
    }

    pub fn stop_voice(&self) {
        self.voice_sink.stop();
    }

    pub fn pause_voice(&self) {
        self.voice_sink.pause();
    }

    pub fn resume_voice(&self) {
        self.voice_sink.play();
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
        self.music_sink.set_volume(settings.master_volume * settings.music_volume);
        self.voice_sink.set_volume(settings.master_volume * settings.voice_volume);
    }

    // System Update ---------------------------------------------
    pub fn update(&self, delta_time: Duration) {
        self.update_fade(delta_time);
        self.cleanup_finished_sfx();
    }

    fn update_fade(&self, delta_time: Duration) {
        let mut fade_state = self.fade_state.write();
        match &mut *fade_state {
            FadeState::FadingIn { target_volume, duration, elapsed } => {
                *elapsed += delta_time;
                let progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0);
                let volume = progress * target_volume;
                self.music_sink.set_volume(volume);
                
                if progress >= 1.0 {
                    *fade_state = FadeState::Idle;
                }
            }
            FadeState::FadingOut { duration, elapsed } => {
                *elapsed += delta_time;
                let progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0);
                let volume = (1.0 - progress) * self.settings.read().music_volume;
                self.music_sink.set_volume(volume);
                
                if progress >= 1.0 {
                    self.music_sink.stop();
                    *fade_state = FadeState::Idle;
                }
            }
            FadeState::Crossfading { out_duration, in_duration, out_elapsed, next_track, next_settings } => {
                *out_elapsed += delta_time;
                let out_progress = (out_elapsed.as_secs_f32() / out_duration.as_secs_f32()).min(1.0);
                let volume = (1.0 - out_progress) * self.settings.read().music_volume;
                self.music_sink.set_volume(volume);
                
                if out_progress >= 1.0 {
                    if let Err(e) = self._play_music(next_track, next_settings.clone(), *in_duration) {
                        log::error!("Failed to play next track: {}", e);
                    }
                    *fade_state = FadeState::Idle;
                }
            }
            FadeState::Idle => {}
        }
    }

    fn cleanup_finished_sfx(&self) {
        let mut controller = self.sfx_controller.lock().unwrap();
        controller.remove_finished();
    }

    // Utility Methods -------------------------------------------
    fn load_audio_file(&self, path: &Path) -> Result<Buffered<SamplesConverter<Decoder<BufReader<File>>, f32>>, AudioError> {
        if let Some(cached) = self.sfx_cache.read().get(path) {
            return Ok(cached.clone());
        }

        let file = File::open(path).map_err(|_| AudioError::FileNotFound(path.to_path_buf()))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|_| AudioError::InvalidFormat)?;
        
        let converted = source.convert_samples::<f32>();
        Ok(converted.buffered())
    }

    pub fn current_music(&self) -> Option<(PathBuf, MusicSettings)> {
        self.current_music.read().clone()
    }

    pub fn music_queue(&self) -> Vec<(PathBuf, MusicSettings)> {
        self.music_queue.read().iter().cloned().collect()
    }

    pub fn clear_music_queue(&self) {
        self.music_queue.write().clear();
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
    AUDIO_SYSTEM.read_recursive()
        .as_ref()
        .map(|_| AUDIO_SYSTEM.read_recursive())
        .ok_or(AudioError::NotInitialized)
}

pub fn get_audio_mut() -> Result<RwLockWriteGuard<'static, Option<AudioPlayer>>, AudioError> {
    AUDIO_SYSTEM.write()
        .as_ref()
        .map(|_| AUDIO_SYSTEM.write())
        .ok_or(AudioError::NotInitialized)
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

// 8. Spatial Audio Helpers =====================================

#[derive(Debug, Clone, Copy)]
pub struct AudioPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub max_distance: f32,
}

impl Default for AudioPosition {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            max_distance: 100.0,
        }
    }
}

pub fn calculate_spatial_volume(
    listener: AudioPosition,
    emitter: AudioPosition,
) -> f32 {
    let dx = emitter.x - listener.x;
    let dy = emitter.y - listener.y;
    let dz = emitter.z - listener.z;
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    
    (1.0 - (distance / emitter.max_distance).clamp(0.0, 1.0)).powi(2)
}

// 9. Filter Implementations ====================================

trait AudioFilter: Source<Item = f32> + Send + 'static {
    fn apply(&mut self, sample: f32) -> f32;
}

struct LowPassFilter<S: Source<Item = f32>> {
    source: S,
    cutoff: f32,
    prev_sample: f32,
    sample_rate: u32,
}

impl<S: Source<Item = f32>> LowPassFilter<S> {
    fn new(source: S, cutoff: f32, sample_rate: u32) -> Self {
        Self {
            source,
            cutoff,
            prev_sample: 0.0,
            sample_rate,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for LowPassFilter<S> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.source.next().map(|sample| {
            let rc = 1.0 / (2.0 * std::f32::consts::PI * self.cutoff);
            let dt = 1.0 / self.sample_rate as f32;
            let alpha = dt / (rc + dt);
            let filtered = alpha * sample + (1.0 - alpha) * self.prev_sample;
            self.prev_sample = filtered;
            filtered
        })
    }
}

impl<S: Source<Item = f32>> Source for LowPassFilter<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.source.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

struct HighPassFilter<S: Source<Item = f32>> {
    source: S,
    cutoff: f32,
    prev_sample: f32,
    prev_output: f32,
    sample_rate: u32,
}

impl<S: Source<Item = f32>> HighPassFilter<S> {
    fn new(source: S, cutoff: f32, sample_rate: u32) -> Self {
        Self {
            source,
            cutoff,
            prev_sample: 0.0,
            prev_output: 0.0,
            sample_rate,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for HighPassFilter<S> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.source.next().map(|sample| {
            let rc = 1.0 / (2.0 * std::f32::consts::PI * self.cutoff);
            let dt = 1.0 / self.sample_rate as f32;
            let alpha = rc / (rc + dt);
            let output = alpha * (self.prev_output + sample - self.prev_sample);
            self.prev_sample = sample;
            self.prev_output = output;
            output
        })
    }
}

impl<S: Source<Item = f32>> Source for HighPassFilter<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.source.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

// 10. Reverb Implementation ===================================

struct ReverbEffect<S: Source<Item = f32>> {
    source: S,
    buffers: [Vec<f32>; 4],
    buffer_idx: usize,
    wet: f32,
    decay: f32,
}

impl<S: Source<Item = f32>> ReverbEffect<S> {
    fn new(source: S, wet: f32, decay: f32) -> Self {
        Self {
            source,
            buffers: [
                vec![0.0; 4410], // 100ms at 44.1kHz
                vec![0.0; 3087], // 70ms
                vec![0.0; 2205], // 50ms
                vec![0.0; 1764], // 40ms
            ],
            buffer_idx: 0,
            wet,
            decay,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for ReverbEffect<S> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.source.next().map(|sample| {
            let mut reverb = 0.0;
            for buf in &mut self.buffers {
                reverb += buf[self.buffer_idx] * self.decay;
                buf[self.buffer_idx] = sample + reverb * self.wet;
            }
            
            self.buffer_idx = (self.buffer_idx + 1) % self.buffers[0].len();
            sample + reverb
        })
    }
}

impl<S: Source<Item = f32>> Source for ReverbEffect<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.source.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

// 11. Test Sounds =============================================

pub fn play_test_sound() -> Result<(), AudioError> {
    let audio = get_audio()?;
    let sink = Sink::try_new(&audio.stream_handle)
        .map_err(|_| AudioError::DeviceError)?;
    
    let source = SineWave::new(440.0)
        .take_duration(Duration::from_secs_f32(0.5))
        .amplify(0.2);
    
    sink.append(source);
    Ok(())
}
