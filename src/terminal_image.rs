//! Owned image data copied at the libghostty-vt boundary.
use std::{collections::HashMap, ffi::c_void, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalImage {
    pub id: u32,
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<[u8]>,
    pub col: i32,
    pub row: i32,
    pub z: i32,
    pub offset_x: u32,
    pub offset_y: u32,
    pub display_width: u32,
    pub display_height: u32,
    pub source_x: u32,
    pub source_y: u32,
    pub source_width: u32,
    pub source_height: u32,
}

#[repr(C)]
struct RawImage {
    generation: u64,
    width: u32,
    height: u32,
    pixels: *const u8,
    pixels_len: usize,
    format: i32,
    col: i32,
    row: i32,
    z: i32,
    offset_x: u32,
    offset_y: u32,
    display_width: u32,
    display_height: u32,
    source_x: u32,
    source_y: u32,
    source_width: u32,
    source_height: u32,
    id: u32,
}

#[repr(C)]
struct DecodedImage {
    width: u32,
    height: u32,
    data: *mut u8,
    data_len: usize,
}

#[repr(C)]
struct DecodedFrame {
    data: *mut u8,
    data_len: usize,
    gap_ms: u32,
}
#[repr(C)]
struct DecodedAnimation {
    width: u32,
    height: u32,
    frames: *mut DecodedFrame,
    frames_len: usize,
    loops: u32,
    source: *mut c_void,
    source_bytes: usize,
    source_next: Option<unsafe extern "C" fn(*mut c_void, *const c_void, *mut DecodedFrame) -> i32>,
    source_free: Option<unsafe extern "C" fn(*mut c_void)>,
}

// The image crate's frame iterator is not Send. Construct and keep it on its
// worker; only owned RGBA frames cross the channel. Two queued frames bound
// decode-ahead work, and dropping the receiver cancels a blocked producer.
struct DeferredFrames {
    receiver: std::sync::mpsc::Receiver<Result<image::Frame, String>>,
    #[cfg(test)]
    finished: Arc<std::sync::atomic::AtomicBool>,
}

static IMAGE_WORKERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
struct ImageWorkerPermit {
    #[cfg(test)]
    finished: Arc<std::sync::atomic::AtomicBool>,
}
impl Drop for ImageWorkerPermit {
    fn drop(&mut self) {
        IMAGE_WORKERS.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        #[cfg(test)]
        self.finished
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

fn start_gif(bytes: &[u8]) -> Result<Option<(image::Frame, u32, DeferredFrames)>, String> {
    use image::{AnimationDecoder, ImageDecoder};
    use std::sync::{atomic::Ordering, mpsc};
    let mut count = IMAGE_WORKERS.load(Ordering::Acquire);
    loop {
        if count >= 8 {
            return Ok(None);
        }
        match IMAGE_WORKERS.compare_exchange_weak(
            count,
            count + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,
            Err(actual) => count = actual,
        }
    }
    #[cfg(test)]
    let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let permit = ImageWorkerPermit {
        #[cfg(test)]
        finished: Arc::clone(&finished),
    };
    let bytes = bytes.to_vec();
    let (initial_tx, initial_rx) = mpsc::sync_channel(1);
    let (tx, receiver) = mpsc::sync_channel(2);
    std::thread::Builder::new()
        .name("terminal-image-decoder".into())
        .spawn(move || {
            let _permit = permit;
            let mut decoder = match image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes))
            {
                Ok(decoder) => decoder,
                Err(error) => {
                    let _ = initial_tx.send(Err(error.to_string()));
                    return;
                }
            };
            if let Err(error) = decoder.set_limits(image_limits()) {
                let _ = initial_tx.send(Err(error.to_string()));
                return;
            }
            let loops = loop_count(decoder.loop_count());
            let mut frames = decoder.into_frames();
            let first = match frames.next() {
                Some(Ok(frame)) => frame,
                Some(Err(error)) => {
                    let _ = initial_tx.send(Err(error.to_string()));
                    return;
                }
                None => {
                    let _ = initial_tx.send(Err("Empty animation".into()));
                    return;
                }
            };
            let dimensions = first.buffer().dimensions();
            let mut total = first.buffer().len();
            if total == 0 || total > 64 * 1024 * 1024 {
                let _ = initial_tx.send(Err("Image exceeds storage limit".into()));
                return;
            }
            if initial_tx.send(Ok((first, loops))).is_err() {
                return;
            }
            for (index, frame) in frames.enumerate() {
                let result = frame.map_err(|error| error.to_string()).and_then(|frame| {
                    if index >= 4095
                        || frame.buffer().dimensions() != dimensions
                        || frame.buffer().len() > (256 * 1024 * 1024usize).saturating_sub(total)
                    {
                        return Err("Animation exceeds storage limit".into());
                    }
                    total += frame.buffer().len();
                    Ok(frame)
                });
                let failed = result.is_err();
                if tx.send(result).is_err() || failed {
                    break;
                }
            }
        })
        .map_err(|error| error.to_string())?;
    let (first, loops) = initial_rx.recv().map_err(|error| error.to_string())??;
    Ok(Some((
        first,
        loops,
        DeferredFrames {
            receiver,
            #[cfg(test)]
            finished,
        },
    )))
}

unsafe extern "C" fn next_deferred_frame(
    source: *mut c_void,
    allocator: *const c_void,
    out: *mut DecodedFrame,
) -> i32 {
    std::panic::catch_unwind(|| {
        let source = unsafe { &*source.cast::<DeferredFrames>() };
        match source.receiver.try_recv() {
            Ok(Ok(frame)) => match copy_frame(allocator, &frame) {
                Some(frame) => {
                    unsafe { out.write(frame) };
                    1
                }
                None => -1,
            },
            Ok(Err(_)) => -1,
            Err(std::sync::mpsc::TryRecvError::Empty) => 0,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => 2,
        }
    })
    .unwrap_or(-1)
}

unsafe extern "C" fn free_deferred_frames(source: *mut c_void) {
    drop(unsafe { Box::from_raw(source.cast::<DeferredFrames>()) });
}

fn copy_frame(allocator: *const c_void, frame: &image::Frame) -> Option<DecodedFrame> {
    let rgba = frame.buffer();
    let pixels = unsafe { ghostty_alloc(allocator, rgba.len()) };
    if pixels.is_null() {
        return None;
    }
    let (num, den) = frame.delay().numer_denom_ms();
    let gap_ms = u64::from(num)
        .div_ceil(u64::from(den).max(1))
        .clamp(10, u64::from(u32::MAX)) as u32;
    unsafe { std::ptr::copy_nonoverlapping(rgba.as_ptr(), pixels, rgba.len()) };
    Some(DecodedFrame {
        data: pixels,
        data_len: rgba.len(),
        gap_ms,
    })
}

unsafe extern "C" {
    fn ghostty_alloc(allocator: *const c_void, size: usize) -> *mut u8;
    fn ghostty_free(allocator: *const c_void, data: *mut u8, size: usize);
    fn spike_images_init();
    fn spike_terminal_image_media(terminal: *mut c_void, directory: *const u8, len: usize) -> i32;
    fn spike_terminal_images(
        terminal: *mut c_void,
        context: *mut c_void,
        callback: unsafe extern "C" fn(*mut c_void, *const RawImage),
    ) -> i32;
}

pub(crate) fn init() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| unsafe { spike_images_init() });
}

pub(crate) fn configure_media(terminal: *mut c_void) -> Result<(), String> {
    let path = std::env::temp_dir();
    #[cfg(unix)]
    let directory = {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes()
    };
    #[cfg(not(unix))]
    let directory = path
        .to_str()
        .ok_or("temporary directory must be valid UTF-8")?
        .as_bytes();
    let result =
        unsafe { spike_terminal_image_media(terminal, directory.as_ptr(), directory.len()) };
    if result == 0 {
        Ok(())
    } else {
        Err("could not enable terminal image media".into())
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn agent_decode_png(
    _: *mut c_void,
    allocator: *const c_void,
    data: *const u8,
    len: usize,
    out: *mut DecodedImage,
) -> bool {
    decode_image(allocator, data, len, out, Some(image::ImageFormat::Png))
}

#[unsafe(no_mangle)]
unsafe extern "C" fn agent_decode_image(
    _: *mut c_void,
    allocator: *const c_void,
    data: *const u8,
    len: usize,
    out: *mut DecodedAnimation,
) -> bool {
    std::panic::catch_unwind(|| {
        if data.is_null() || out.is_null() || len > 64 * 1024 * 1024 {
            return false;
        }
        let bytes = unsafe { std::slice::from_raw_parts(data, len) };
        let deferred = if image::guess_format(bytes).ok() == Some(image::ImageFormat::Gif) {
            match start_gif(bytes) {
                Ok(result) => result,
                Err(_) => return false,
            }
        } else {
            None
        };
        let (result, source) = match deferred {
            Some((first, loops, source)) => (Ok((vec![first], loops)), Some(source)),
            None => (decode_frames(bytes), None),
        };
        let Ok((frames, loops)) = result else {
            return false;
        };
        let Some(first) = frames.first() else {
            return false;
        };
        let (width, height) = first.buffer().dimensions();
        // The allocation API returns byte-aligned storage. Use unaligned writes;
        // the corresponding Zig wrapper reads it with alignment 1 as well.
        let array_bytes = frames.len() * std::mem::size_of::<DecodedFrame>();
        let mut copied = Vec::with_capacity(frames.len());
        let raw = unsafe { ghostty_alloc(allocator, array_bytes) }.cast::<DecodedFrame>();
        if raw.is_null() {
            return false;
        }
        for (index, frame) in frames.iter().enumerate() {
            let rgba = frame.buffer();
            let pixels = unsafe { ghostty_alloc(allocator, rgba.len()) };
            if pixels.is_null() {
                for (pointer, len) in copied {
                    unsafe { ghostty_free(allocator, pointer, len) };
                }
                unsafe { ghostty_free(allocator, raw.cast(), array_bytes) };
                return false;
            }
            copied.push((pixels, rgba.len()));
            let (num, den) = frame.delay().numer_denom_ms();
            let gap_ms = (u64::from(num).div_ceil(u64::from(den).max(1)))
                .clamp(10, u64::from(u32::MAX)) as u32;
            unsafe {
                std::ptr::copy_nonoverlapping(rgba.as_ptr(), pixels, rgba.len());
                raw.add(index).write_unaligned(DecodedFrame {
                    data: pixels,
                    data_len: rgba.len(),
                    gap_ms,
                });
            }
        }
        unsafe {
            out.write(DecodedAnimation {
                width,
                height,
                frames: raw,
                frames_len: frames.len(),
                loops,
                source_bytes: if source.is_some() {
                    len + first.buffer().len() * 5
                } else {
                    0
                },
                source: source.map_or(std::ptr::null_mut(), |source| {
                    Box::into_raw(Box::new(source)).cast()
                }),
                source_next: Some(next_deferred_frame),
                source_free: Some(free_deferred_frames),
            });
        }
        true
    })
    .unwrap_or(false)
}

fn image_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(64 * 1024 * 1024);
    limits
}

fn decode_frames(bytes: &[u8]) -> Result<(Vec<image::Frame>, u32), String> {
    use image::{AnimationDecoder, ImageDecoder};
    let format = image::guess_format(bytes).map_err(|e| e.to_string())?;
    let reader = std::io::Cursor::new(bytes);
    match format {
        image::ImageFormat::Gif => {
            let mut decoder =
                image::codecs::gif::GifDecoder::new(reader).map_err(|e| e.to_string())?;
            decoder
                .set_limits(image_limits())
                .map_err(|e| e.to_string())?;
            return collect_frames(decoder.loop_count(), decoder.into_frames());
        }
        image::ImageFormat::Png => {
            let decoder = image::codecs::png::PngDecoder::with_limits(reader, image_limits())
                .map_err(|e| e.to_string())?;
            if decoder.is_apng().map_err(|e| e.to_string())? {
                let decoder = decoder.apng().map_err(|e| e.to_string())?;
                return collect_frames(decoder.loop_count(), decoder.into_frames());
            }
        }
        image::ImageFormat::WebP => {
            let mut decoder =
                image::codecs::webp::WebPDecoder::new(reader).map_err(|e| e.to_string())?;
            decoder
                .set_limits(image_limits())
                .map_err(|e| e.to_string())?;
            if decoder.has_animation() {
                return collect_frames(decoder.loop_count(), decoder.into_frames());
            }
        }
        _ => {}
    }
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    reader.limits(image_limits());
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut decoded = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    if u64::from(decoded.width()) * u64::from(decoded.height()) * 4 > 64 * 1024 * 1024 {
        return Err("Image exceeds storage limit".into());
    }
    decoded.apply_orientation(orientation);
    Ok((vec![image::Frame::new(decoded.into_rgba8())], 1))
}

fn collect_frames(
    loops: image::metadata::LoopCount,
    frames: image::Frames<'_>,
) -> Result<(Vec<image::Frame>, u32), String> {
    let loops = loop_count(loops);
    let mut result: Vec<image::Frame> = Vec::new();
    let mut total = 0usize;
    for frame in frames {
        let frame = frame.map_err(|e| e.to_string())?;
        let buffer = frame.buffer();
        if buffer.is_empty()
            || buffer.len() > (256 * 1024 * 1024usize).saturating_sub(total)
            || result.len() >= 4096
        {
            return Err("Animation exceeds storage limit".into());
        }
        if result
            .first()
            .is_some_and(|first| first.buffer().dimensions() != buffer.dimensions())
        {
            return Err("Animation has inconsistent canvas dimensions".into());
        }
        total += buffer.len();
        result.push(frame);
    }
    Ok((result, loops))
}

fn loop_count(loops: image::metadata::LoopCount) -> u32 {
    match loops {
        image::metadata::LoopCount::Infinite => 0,
        image::metadata::LoopCount::Finite(n) => n.get(),
    }
}

fn decode_image(
    allocator: *const c_void,
    data: *const u8,
    len: usize,
    out: *mut DecodedImage,
    format: Option<image::ImageFormat>,
) -> bool {
    // No Rust panic may cross the C callback boundary.
    std::panic::catch_unwind(|| {
        if data.is_null() || out.is_null() || len > 64 * 1024 * 1024 {
            return false;
        }
        let bytes = unsafe { std::slice::from_raw_parts(data, len) };
        let format = match format.or_else(|| image::guess_format(bytes).ok()) {
            Some(format) => format,
            None => return false,
        };
        let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(16384);
        limits.max_image_height = Some(16384);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        let Ok(decoded) = reader.decode() else {
            return false;
        };
        if u64::from(decoded.width()) * u64::from(decoded.height()) * 4 > 64 * 1024 * 1024 {
            return false;
        }
        let rgba = decoded.into_rgba8();
        let pixels = unsafe { ghostty_alloc(allocator, rgba.len()) };
        if pixels.is_null() {
            return false;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(rgba.as_ptr(), pixels, rgba.len());
            *out = DecodedImage {
                width: rgba.width(),
                height: rgba.height(),
                data: pixels,
                data_len: rgba.len(),
            };
        }
        true
    })
    .unwrap_or(false)
}

#[derive(Default)]
pub(crate) struct ImageCache {
    pixels: HashMap<u64, Arc<[u8]>>,
    placements: Vec<TerminalImage>,
    failed: bool,
}

impl ImageCache {
    pub(crate) fn snapshot(&mut self, terminal: *mut c_void) -> Result<Vec<TerminalImage>, String> {
        self.placements.clear();
        self.failed = false;
        let result =
            unsafe { spike_terminal_images(terminal, std::ptr::from_mut(self).cast(), collect) };
        if result != 0 || self.failed {
            return Err("could not snapshot terminal images".into());
        }
        self.pixels.retain(|generation, _| {
            self.placements
                .iter()
                .any(|image| image.generation == *generation)
        });
        self.placements.sort_by_key(|image| (image.z, image.id));
        Ok(self.placements.clone())
    }
}

unsafe extern "C" fn collect(context: *mut c_void, raw: *const RawImage) {
    let cache = unsafe { &mut *context.cast::<ImageCache>() };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = unsafe { &*raw };
        let channels = match raw.format {
            0 => 3,
            1 => 4,
            _ => {
                cache.failed = true;
                return;
            }
        };
        let expected = u64::from(raw.width) * u64::from(raw.height) * channels;
        if raw.pixels.is_null() || expected != raw.pixels_len as u64 || expected > 64 * 1024 * 1024
        {
            cache.failed = true;
            return;
        }
        let pixels = cache
            .pixels
            .entry(raw.generation)
            .or_insert_with(|| {
                let source = unsafe { std::slice::from_raw_parts(raw.pixels, raw.pixels_len) };
                if channels == 4 {
                    Arc::from(source)
                } else {
                    source
                        .as_chunks::<3>()
                        .0
                        .iter()
                        .flat_map(|p| [p[0], p[1], p[2], 255])
                        .collect::<Vec<_>>()
                        .into()
                }
            })
            .clone();
        cache.placements.push(TerminalImage {
            id: raw.id,
            generation: raw.generation,
            width: raw.width,
            height: raw.height,
            rgba: pixels,
            col: raw.col,
            row: raw.row,
            z: raw.z,
            offset_x: raw.offset_x,
            offset_y: raw.offset_y,
            display_width: raw.display_width,
            display_height: raw.display_height,
            source_x: raw.source_x,
            source_y: raw.source_y,
            source_width: raw.source_width,
            source_height: raw.source_height,
        });
    }));
    if result.is_err() {
        cache.failed = true;
    }
}

#[cfg(test)]
mod tests {
    use crate::ghostty::Terminal;
    use base64::Engine;

    #[cfg(unix)]
    #[test]
    fn text_session_accepts_non_utf8_temporary_directory() {
        const CHILD: &str = "AGENT_TERMINAL_TEST_NON_UTF8_TMPDIR_CHILD";
        if std::env::var_os(CHILD).is_some() {
            assert!(std::env::temp_dir().to_str().is_none());
            let mut terminal = Terminal::new(20, 10).unwrap();
            terminal.feed(b"ordinary text").unwrap();
            let snapshot = terminal.render_update(true).unwrap();
            let text: String = snapshot
                .cells
                .iter()
                .filter(|cell| cell.y == 0)
                .map(|cell| cell.text.as_str())
                .collect();
            assert!(text.starts_with("ordinary text"), "{text:?}");
            return;
        }

        use std::os::unix::ffi::OsStringExt;
        let mut name = format!(
            "terminal-media-test-{}-{}-",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
        .into_bytes();
        name.push(0xff);
        let directory = std::env::temp_dir().join(std::ffi::OsString::from_vec(name));
        std::fs::create_dir(&directory).unwrap();
        // A child process keeps TMPDIR changes out of parallel parent tests.
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "terminal_image::tests::text_session_accepts_non_utf8_temporary_directory",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("TMPDIR", &directory)
            .output();
        std::fs::remove_dir(&directory).unwrap();
        let output = result.unwrap();
        assert!(
            output.status.success(),
            "child test failed: {}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn deferred_gif_preserves_frames_and_cancels_decode_ahead() {
        let mut bytes = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
            encoder
                .set_repeat(image::codecs::gif::Repeat::Finite(3))
                .unwrap();
            encoder
                .encode_frames((0..16).map(|index| {
                    image::Frame::from_parts(
                        image::RgbaImage::from_fn(32, 32, |x, y| {
                            image::Rgba([
                                if x < 16 { 255 } else { 0 },
                                index * 16,
                                0,
                                if y < 16 && index % 2 == 0 { 0 } else { 255 },
                            ])
                        }),
                        0,
                        0,
                        image::Delay::from_numer_denom_ms(20 + u32::from(index) * 10, 1),
                    )
                }))
                .unwrap();
        }
        let (expected, expected_loops) = super::decode_frames(&bytes).unwrap();
        let (first, loops, source) = super::start_gif(&bytes).unwrap().unwrap();
        assert_eq!(loops, expected_loops);
        assert_eq!(first.buffer(), expected[0].buffer());
        assert_eq!(first.delay(), expected[0].delay());
        for frame in &expected[1..] {
            let actual = source
                .receiver
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
                .unwrap();
            assert_eq!(actual.buffer(), frame.buffer());
            assert_eq!(actual.delay(), frame.delay());
        }
        assert!(matches!(
            source
                .receiver
                .recv_timeout(std::time::Duration::from_secs(2)),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
        ));
        drop(source);

        let (_, _, source) = super::start_gif(&bytes).unwrap().unwrap();
        let finished = std::sync::Arc::clone(&source.finished);
        drop(source); // The producer may be blocked on its bounded queue.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !finished.load(std::sync::atomic::Ordering::Acquire) {
            assert!(
                std::time::Instant::now() < deadline,
                "decoder survived cancellation"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fn tick_when_ready(terminal: &mut Terminal, now: u64) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !terminal.tick_images(now).unwrap() {
            assert!(
                std::time::Instant::now() < deadline,
                "animation frame not ready"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    #[ignore = "requires IMAGE_PROTOCOL_TEST_FILE for a local decoder/VT timing run"]
    fn profile_image_protocol_input() {
        let bytes = std::fs::read(std::env::var("IMAGE_PROTOCOL_TEST_FILE").unwrap()).unwrap();
        let start = std::time::Instant::now();
        let (frames, _) = super::decode_frames(&bytes).unwrap();
        eprintln!(
            "Decode {} bytes / {} frames: {:?}",
            bytes.len(),
            frames.len(),
            start.elapsed()
        );
        drop(frames);
        let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let mut terminal = Terminal::new(100, 80).unwrap();
        terminal.resize(100, 80, 10, 20).unwrap();
        let start = std::time::Instant::now();
        terminal
            .feed(b"\x1b]1337;File=inline=1;width=240px:")
            .unwrap();
        for chunk in data.as_bytes().chunks(16384) {
            terminal.feed(chunk).unwrap();
        }
        let parsed = start.elapsed();
        terminal.feed(b"\x07").unwrap();
        let snapshot = terminal.snapshot().unwrap();
        assert_eq!(snapshot.images.len(), 1);
        eprintln!(
            "VT payload: {:?}; finish/decode/snapshot: {:?}; total: {:?}",
            parsed,
            start.elapsed() - parsed,
            start.elapsed()
        );
    }

    fn png() -> String {
        let pixels = image::RgbaImage::from_pixel(2, 1, image::Rgba([255, 0, 0, 128]));
        let mut encoded = std::io::Cursor::new(Vec::new());
        pixels
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        base64::engine::general_purpose::STANDARD.encode(encoded.into_inner())
    }

    #[test]
    fn kitty_file_and_temporary_file_media_respect_offsets() {
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        let path = std::env::temp_dir().join(format!(
            "tty-graphics-protocol-{}-{}.bin",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, [42, 42, 255, 0, 0, 99, 99]).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(path.to_str().unwrap());
        let reply = terminal
            .feed(format!("\x1b_Ga=T,t=f,f=24,s=1,v=1,i=20,O=2,S=3;{encoded}\x1b\\").as_bytes())
            .unwrap();
        assert!(String::from_utf8_lossy(&reply).contains("OK"), "{reply:?}");
        assert!(path.exists());
        assert_eq!(
            &*terminal.snapshot().unwrap().images[0].rgba,
            &[255, 0, 0, 255]
        );
        let reply = terminal
            .feed(format!("\x1b_Ga=T,t=t,f=24,s=1,v=1,i=21,O=2,S=3;{encoded}\x1b\\").as_bytes())
            .unwrap();
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
            panic!("temporary image was not removed: {reply:?}");
        }
        assert!(String::from_utf8_lossy(&reply).contains("OK"), "{reply:?}");
    }

    #[cfg(windows)]
    #[test]
    fn kitty_windows_shared_memory_reads_requested_span() {
        unsafe extern "system" {
            fn CreateFileMappingW(
                file: *mut std::ffi::c_void,
                attributes: *const std::ffi::c_void,
                protect: u32,
                high: u32,
                low: u32,
                name: *const u16,
            ) -> *mut std::ffi::c_void;
            fn MapViewOfFile(
                mapping: *mut std::ffi::c_void,
                access: u32,
                high: u32,
                low: u32,
                len: usize,
            ) -> *mut u8;
            fn UnmapViewOfFile(view: *const std::ffi::c_void) -> i32;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }
        let name = format!("Local\\tty-graphics-protocol-{}", std::process::id());
        let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
        let mapping = unsafe {
            CreateFileMappingW(
                -1isize as *mut _,
                std::ptr::null(),
                4,
                0,
                4096,
                wide.as_ptr(),
            )
        };
        assert!(!mapping.is_null());
        let view = unsafe { MapViewOfFile(mapping, 2, 0, 0, 4096) };
        assert!(!view.is_null());
        unsafe {
            std::ptr::copy_nonoverlapping([7u8, 0, 255, 0].as_ptr(), view, 4);
        }
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&name);
        let result = terminal
            .feed(format!("\x1b_Ga=T,t=s,f=24,s=1,v=1,i=25,O=1,S=3;{encoded}\x1b\\").as_bytes());
        unsafe {
            UnmapViewOfFile(view.cast());
            CloseHandle(mapping);
        }
        let reply = result.unwrap();
        assert!(String::from_utf8_lossy(&reply).contains("OK"), "{reply:?}");
        assert_eq!(
            &*terminal.snapshot().unwrap().images[0].rgba,
            &[0, 255, 0, 255]
        );
    }

    #[test]
    fn kitty_unicode_placeholders_resolve_into_visible_image_fragments() {
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        terminal
            .feed(b"\x1b_Ga=T,f=24,s=2,v=1,i=42,U=1,c=2,r=1;/wAAAP8A\x1b\\")
            .unwrap();
        assert!(terminal.snapshot().unwrap().images.is_empty());
        terminal
            .feed(
                "\x1b[38;5;42m\u{10eeee}\u{0305}\u{0305}\u{10eeee}\u{0305}\u{030d}\x1b[39m"
                    .as_bytes(),
            )
            .unwrap();
        let snapshot = terminal.snapshot().unwrap();
        assert_eq!(snapshot.images.len(), 1);
        let image = &snapshot.images[0];
        assert_eq!((image.col, image.row, image.z), (0, 0, -1));
        assert_eq!(
            (image.display_width, image.display_height, image.offset_y),
            (16, 8, 4)
        );
        terminal.feed(b"\x1b[2J").unwrap();
        assert!(terminal.snapshot().unwrap().images.is_empty());
    }

    #[test]
    fn iterm_animated_gif_keeps_frames_delays_and_disposal() {
        let mut bytes = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
            encoder
                .set_repeat(image::codecs::gif::Repeat::Infinite)
                .unwrap();
            encoder
                .encode_frames(
                    [[255, 0, 0, 255], [0, 0, 255, 255]]
                        .into_iter()
                        .map(|color| {
                            image::Frame::from_parts(
                                image::RgbaImage::from_pixel(2, 2, image::Rgba(color)),
                                0,
                                0,
                                image::Delay::from_numer_denom_ms(30, 1),
                            )
                        }),
                )
                .unwrap();
        }
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        terminal
            .feed(format!("\x1b]1337;File=inline=1:{data}\x07").as_bytes())
            .unwrap();
        let first = terminal.snapshot().unwrap();
        assert_eq!(&first.images[0].rgba[..4], &[255, 0, 0, 255]);
        assert!(!terminal.tick_images(29).unwrap());
        tick_when_ready(&mut terminal, 30);
        let second = terminal.snapshot().unwrap();
        assert_eq!(&second.images[0].rgba[..4], &[0, 0, 255, 255]);
        assert_ne!(first.images[0].generation, second.images[0].generation);
        tick_when_ready(&mut terminal, 60);
        assert_eq!(
            &terminal.snapshot().unwrap().images[0].rgba[..4],
            &[255, 0, 0, 255]
        );
        assert_eq!(
            terminal.feed(b"\x1b]1337;Capabilities\x1b\\").unwrap(),
            b"\x1b]1337;Capabilities=F\x1b\\"
        );
    }

    #[test]
    fn kitty_deletion_selectors_match_rectangles_and_ranges() {
        for selector in [
            "d=p,x=5,y=3",
            "d=q,x=5,y=3,z=7",
            "d=c",
            "d=x,x=5",
            "d=y,y=3",
            "d=z,z=7",
            "d=r,x=2,y=2",
            "d=n,I=22",
        ] {
            let mut terminal = Terminal::new(20, 10).unwrap();
            terminal.resize(20, 10, 8, 16).unwrap();
            let target = if selector.starts_with("d=n") {
                "I=22"
            } else {
                "i=2"
            };
            terminal.feed(format!("\x1b_Ga=T,f=24,s=1,v=1,i=1,C=1;/wAA\x1b\\\x1b[2;5H\x1b_Ga=T,f=24,s=1,v=1,{target},c=2,r=3,z=7,C=1;AP8A\x1b\\\x1b[3;5H").as_bytes()).unwrap();
            terminal
                .feed(format!("\x1b_Ga=d,{selector}\x1b\\").as_bytes())
                .unwrap();
            let images = terminal.snapshot().unwrap().images;
            assert_eq!(images.len(), 1, "{selector}");
            assert_eq!(images[0].id, 1, "{selector}");
        }
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        terminal.feed(b"\x1b[2;5H\x1b_Ga=T,f=24,s=1,v=1,i=5,c=2,r=3,C=1;/wAA\x1b\\\x1b_Ga=d,d=p,x=1,y=3\x1b\\").unwrap();
        assert_eq!(
            terminal.snapshot().unwrap().images.len(),
            1,
            "a cell outside the image rectangle must not delete it"
        );
    }

    #[test]
    fn kitty_layout_compression_queries_and_screen_lifetimes() {
        use std::io::Write;
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        assert_eq!(
            terminal.feed(b"\x1b[16t\x1b[18t").unwrap(),
            b"\x1b[6;16;8t\x1b[8;10;20t"
        );
        let mut compressed =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        compressed.write_all(&[255, 0, 0, 0, 255, 0]).unwrap();
        let data = base64::engine::general_purpose::STANDARD.encode(compressed.finish().unwrap());
        let query = terminal
            .feed(format!("\x1b_Ga=q,f=24,o=z,s=2,v=1,i=123;{data}\x1b\\").as_bytes())
            .unwrap();
        assert_eq!(query, b"\x1b_Gi=123;OK\x1b\\");
        assert!(terminal.snapshot().unwrap().images.is_empty());
        terminal
            .feed(
                format!("\x1b[2;3H\x1b_Ga=T,f=24,o=z,s=2,v=1,I=7,c=4,r=2;{data}\x1b\\").as_bytes(),
            )
            .unwrap();
        let snapshot = terminal.snapshot().unwrap();
        assert_eq!(snapshot.cursor, Some((6, 3)));
        assert_eq!((snapshot.images[0].col, snapshot.images[0].row), (2, 1));
        assert_eq!(
            (
                snapshot.images[0].display_width,
                snapshot.images[0].display_height
            ),
            (32, 32)
        );
        terminal
            .feed(b"\x1b_Ga=p,I=7,p=2,x=1,w=99,h=99,c=2,r=1,X=3,Y=2,C=1,z=-2\x1b\\")
            .unwrap();
        let snapshot = terminal.snapshot().unwrap();
        let crop = &snapshot.images[0];
        assert_eq!(
            (crop.source_x, crop.source_width, crop.source_height),
            (1, 1, 1)
        );
        assert_eq!((crop.offset_x, crop.offset_y), (3, 2));
        terminal.feed(b"\x1b[?1049h").unwrap();
        assert!(terminal.snapshot().unwrap().images.is_empty());
        terminal
            .feed(b"\x1b_Ga=T,f=24,s=1,v=1,i=99,C=1;AAD/\x1b\\")
            .unwrap();
        assert_eq!(terminal.snapshot().unwrap().images.len(), 1);
        terminal.feed(b"\x1b[?1049l").unwrap();
        assert_eq!(terminal.snapshot().unwrap().images.len(), 2);
        terminal.resize(30, 12, 10, 20).unwrap();
        assert_eq!(terminal.snapshot().unwrap().images[0].display_width, 17);
        let invalid = terminal
            .feed(b"\x1b_Ga=q,f=24,s=2,v=1,i=124;/wAA\x1b\\")
            .unwrap();
        assert!(String::from_utf8_lossy(&invalid).contains("EINVAL"));
        let oversized = terminal
            .feed(b"\x1b_Ga=q,i=125,f=24,s=5000,v=5000\x1b\\")
            .unwrap();
        assert!(String::from_utf8_lossy(&oversized).contains("dimensions too large"));
        let invalid = terminal.feed(b"\x1b_Ga=p,i=1,I=7\x1b\\").unwrap();
        assert!(String::from_utf8_lossy(&invalid).contains("EINVAL"));
        assert!(
            terminal
                .feed(b"\x1b_Ga=p,i=1,I=7,q=2\x1b\\")
                .unwrap()
                .is_empty()
        );
        terminal.feed(b"\x1bc").unwrap();
        assert!(terminal.snapshot().unwrap().images.is_empty());
    }

    #[test]
    fn kitty_visible_deletion_keeps_scrollback_and_frame_deletion_renumbers() {
        let mut terminal = Terminal::new(20, 3).unwrap();
        terminal.resize(20, 3, 8, 16).unwrap();
        terminal
            .feed(b"\x1b_Ga=T,f=24,s=1,v=1,i=1,C=1;/wAA\x1b\\x\r\nx\r\nx\r\nx\r\n")
            .unwrap();
        terminal
            .feed(b"\x1b_Ga=T,f=24,s=1,v=1,i=2,C=1;AP8A\x1b\\\x1b_Ga=d,d=A\x1b\\")
            .unwrap();
        assert!(terminal.snapshot().unwrap().images.is_empty());
        terminal.scroll_viewport(isize::MIN).unwrap();
        assert_eq!(terminal.snapshot().unwrap().images[0].id, 1);
        terminal.scroll_viewport(isize::MAX).unwrap();
        terminal.feed(b"\x1b_Ga=p,i=1,C=1\x1b\\\x1b_Ga=f,i=1,f=24,s=1,v=1;AAD/\x1b\\\x1b_Ga=a,i=1,c=2\x1b\\").unwrap();
        terminal.feed(b"\x1b_Ga=d,d=f,i=1,r=1\x1b\\").unwrap();
        assert_eq!(
            &terminal.snapshot().unwrap().images[0].rgba[..4],
            &[0, 0, 255, 255]
        );
        terminal.feed(b"\x1b_Ga=d,d=f,i=1,r=1\x1b\\").unwrap();
        assert_eq!(terminal.snapshot().unwrap().images.len(), 1);
        terminal.feed(b"\x1b_Ga=d,d=F,i=1,r=1\x1b\\").unwrap();
        assert!(terminal.snapshot().unwrap().images.is_empty());
    }

    #[test]
    fn kitty_animation_transfers_composes_and_advances_on_the_terminal_clock() {
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        terminal
            .feed(b"\x1b_Ga=T,f=24,s=2,v=1,i=9,C=1;/wAA/wAA\x1b\\")
            .unwrap();
        let reply = terminal
            .feed(b"\x1b_Ga=f,f=32,s=1,v=1,i=9,x=1,c=1,z=30;AAD/gA==\x1b\\")
            .unwrap();
        assert_eq!(reply, b"\x1b_Gi=9;OK\x1b\\");
        terminal.feed(b"\x1b_Ga=a,i=9,c=2\x1b\\").unwrap();
        assert_eq!(
            &*terminal.snapshot().unwrap().images[0].rgba,
            &[255, 0, 0, 255, 127, 0, 128, 255]
        );
        let error = terminal
            .feed(b"\x1b_Ga=c,i=9,r=2,c=2,w=2,h=1\x1b\\")
            .unwrap();
        assert!(String::from_utf8(error).unwrap().contains("EINVAL"));
        terminal
            .feed(b"\x1b_Ga=c,i=9,r=1,c=2,w=1,h=1,x=0,X=1,C=1\x1b\\")
            .unwrap();
        assert_eq!(
            &*terminal.snapshot().unwrap().images[0].rgba,
            &[255, 0, 0, 255, 255, 0, 0, 255]
        );
        terminal
            .feed(b"\x1b_Ga=f,f=24,s=2,v=1,i=9,r=2,z=30,m=1;AP8A\x1b\\")
            .unwrap();
        let reply = terminal.feed(b"\x1b_Ga=f,m=0;AP8A\x1b\\").unwrap();
        assert_eq!(reply, b"\x1b_Gi=9;OK\x1b\\");
        terminal
            .feed(b"\x1b_Ga=a,i=9,r=1,z=20,c=1,s=3,v=2\x1b\\")
            .unwrap();
        assert!(!terminal.tick_images(19).unwrap());
        assert_eq!(
            &terminal.snapshot().unwrap().images[0].rgba[..4],
            &[255, 0, 0, 255]
        );
        assert!(terminal.tick_images(20).unwrap());
        assert_eq!(
            &terminal.snapshot().unwrap().images[0].rgba[..4],
            &[0, 255, 0, 255]
        );
        terminal.tick_images(100).unwrap();
        assert!(!terminal.tick_images(200).unwrap());
        assert_eq!(
            &terminal.snapshot().unwrap().images[0].rgba[..4],
            &[0, 255, 0, 255]
        );
    }

    #[test]
    fn kitty_relative_placements_follow_parents_and_reject_cycles() {
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        terminal
            .feed(b"\x1b_Ga=T,f=24,s=1,v=1,i=1,p=1,C=1;/wAA\x1b\\")
            .unwrap();
        terminal
            .feed(b"\x1b_Ga=T,f=24,s=1,v=1,i=2,p=2,P=1,Q=1,H=3,V=2;AP8A\x1b\\")
            .unwrap();
        let snapshot = terminal.snapshot().unwrap();
        let child = snapshot.images.iter().find(|i| i.id == 2).unwrap();
        assert_eq!((child.col, child.row), (3, 2));
        assert_eq!(snapshot.cursor, Some((0, 0)));
        let reply = terminal.feed(b"\x1b_Ga=p,i=1,p=1,P=2,Q=2\x1b\\").unwrap();
        assert!(String::from_utf8(reply).unwrap().contains("ECYCLE"));
        terminal
            .feed(b"\x1b[2;2H\x1b_Ga=p,i=1,p=1,C=1\x1b\\")
            .unwrap();
        let snapshot = terminal.snapshot().unwrap();
        let child = snapshot.images.iter().find(|i| i.id == 2).unwrap();
        assert_eq!((child.col, child.row), (4, 3));
        terminal.feed(b"\x1b_Ga=d,d=i,i=1,p=1\x1b\\").unwrap();
        assert!(terminal.snapshot().unwrap().images.is_empty());
        let reply = terminal.feed(b"\x1b_Ga=p,i=2\x1b\\").unwrap();
        assert!(String::from_utf8(reply).unwrap().contains("ENOENT"));
    }

    #[test]
    fn png_decoder_and_iterm_multipart_sizing() {
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        let payload = png();
        terminal
            .feed(format!("\x1b_Ga=T,f=100,i=7;{payload}\x1b\\").as_bytes())
            .unwrap();
        let snapshot = terminal.snapshot().unwrap();
        assert_eq!(snapshot.images.len(), 1);
        assert_eq!(&*snapshot.images[0].rgba, &[255, 0, 0, 128, 255, 0, 0, 128]);
        terminal.feed(b"\x1bc").unwrap();
        terminal
            .feed(b"\x1b]1337;MultipartFile=inline=1;width=50%;height=2;preserveAspectRatio=0\x07")
            .unwrap();
        for chunk in payload.as_bytes().chunks(12) {
            terminal.feed(b"\x1b]1337;FilePart=").unwrap();
            terminal.feed(chunk).unwrap();
            terminal.feed(b"\x1b\\").unwrap();
            assert!(terminal.snapshot().unwrap().images.is_empty());
        }
        terminal.feed(b"\x1b]1337;FileEnd\x07").unwrap();
        let snapshot = terminal.snapshot().unwrap();
        assert_eq!(snapshot.images.len(), 1);
        assert_eq!(snapshot.images[0].display_width, 80);
        assert_eq!(snapshot.images[0].display_height, 32);
        assert_eq!(snapshot.cursor, Some((0, 2)));
    }

    #[test]
    fn iterm_single_sequence_accepts_bel_and_st_at_every_byte_boundary() {
        let payload = png();
        for ending in ["\x07", "\x1b\\"] {
            let mut terminal = Terminal::new(20, 10).unwrap();
            terminal.resize(20, 10, 8, 16).unwrap();
            let sequence = format!("\x1b]1337;File=inline=1;width=32px:{payload}{ending}");
            for byte in sequence.bytes() {
                terminal.feed(&[byte]).unwrap();
            }
            let snapshot = terminal.snapshot().unwrap();
            assert_eq!(snapshot.images.len(), 1);
            assert_eq!(snapshot.images[0].display_width, 32);
            assert_eq!(snapshot.images[0].display_height, 16);
        }
    }

    #[test]
    fn kitty_images_survive_fragmentation_and_replace_and_delete() {
        let mut terminal = Terminal::new(20, 10).unwrap();
        terminal.resize(20, 10, 8, 16).unwrap();
        let mut reply = Vec::new();
        for byte in b"\x1b_Ga=T,f=24,s=1,v=1,i=42;/wAA\x1b\\" {
            reply.extend(terminal.feed(&[*byte]).unwrap());
        }
        assert_eq!(reply, b"\x1b_Gi=42;OK\x1b\\");
        let initial = terminal.snapshot().unwrap();
        assert_eq!(initial.images.len(), 1);
        assert_eq!(&*initial.images[0].rgba, &[255, 0, 0, 255]);
        let generation = initial.images[0].generation;
        terminal
            .feed(b"\x1b_Ga=t,f=32,s=1,v=1,i=42;AP8A/w==\x1b\\")
            .unwrap();
        assert!(terminal.snapshot().unwrap().images.is_empty());
        terminal.feed(b"\x1b_Ga=p,i=42\x1b\\").unwrap();
        let replaced = terminal.snapshot().unwrap();
        assert_ne!(replaced.images[0].generation, generation);
        assert_eq!(&*replaced.images[0].rgba, &[0, 255, 0, 255]);
        terminal.feed(b"\x1b_Ga=d,d=I,i=42\x1b\\").unwrap();
        assert!(terminal.snapshot().unwrap().images.is_empty());
    }
}
