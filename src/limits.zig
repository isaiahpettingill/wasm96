pub const max_cartridge_size: usize = 128 * 1024 * 1024;
pub const max_guest_ram_size: u64 = 128 * 1024 * 1024;
pub const max_sram_size: usize = 64 * 1024 * 1024;

pub const fb_default_width: u32 = 320;
pub const fb_default_height: u32 = 224;
pub const fb_max_width: u32 = 1024;
pub const fb_max_height: u32 = 1024;
pub const fb_max_pixels: usize = fb_max_width * fb_max_height;
pub const fb_max_size: usize = fb_max_pixels * @sizeOf(u32);

pub const audio_sample_rate: usize = 48_000;
pub const video_fps: usize = 60;
pub const audio_channels: usize = 2;
pub const audio_frames_per_video: usize = audio_sample_rate / video_fps;
pub const audio_samples: usize = audio_frames_per_video * audio_channels;
pub const audio_size: usize = audio_samples * @sizeOf(i16);
pub const audio_mapped_size: usize = 4096;

pub const controller_count: usize = 4;
pub const controller_buttons: usize = 12;
pub const controller_size: usize = 3;
pub const controller_mapped_size: usize = 4096;
