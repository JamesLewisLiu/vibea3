//! Isolated x86 harness for behavioral tests against the official AVS property API.
//!
//! Build with:
//! `rustc --target i686-pc-windows-msvc -O tools/official_property_harness.rs`

use std::{
    env,
    ffi::{CString, c_char, c_int, c_void},
    fs,
    os::windows::ffi::OsStrExt,
    path::Path,
    ptr,
    sync::Mutex,
};

type Module = *mut c_void;
type Property = c_void;
type Create = unsafe extern "C" fn(c_int, *mut c_void, c_int) -> *mut Property;
type Destroy = unsafe extern "C" fn(*mut Property);
type FileRead = unsafe extern "C" fn(
    *mut Property,
    c_int,
    unsafe extern "C" fn(c_int, *mut c_void, c_int) -> c_int,
    c_int,
) -> c_int;
type FileWrite = unsafe extern "C" fn(
    *mut Property,
    c_int,
    unsafe extern "C" fn(c_int, *const c_void, c_int) -> c_int,
    c_int,
) -> c_int;
type Mode = unsafe extern "C" fn(c_int, c_int) -> c_int;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> Module;
    fn GetProcAddress(module: Module, name: *const c_char) -> *mut c_void;
    fn FreeLibrary(module: Module) -> c_int;
}

#[derive(Default)]
struct Io {
    input: Vec<u8>,
    read_at: usize,
    output: Vec<u8>,
    read_chunk: usize,
    write_chunk: usize,
    read_fail_after: Option<usize>,
    write_fail_after: Option<usize>,
}

static IO: Mutex<Io> = Mutex::new(Io {
    input: Vec::new(),
    read_at: 0,
    output: Vec::new(),
    read_chunk: usize::MAX,
    write_chunk: usize::MAX,
    read_fail_after: None,
    write_fail_after: None,
});

unsafe extern "C" fn read_callback(_: c_int, dst: *mut c_void, size: c_int) -> c_int {
    if dst.is_null() || size <= 0 {
        return 0;
    }
    let mut io = IO.lock().unwrap();
    if io.read_fail_after.is_some_and(|at| io.read_at >= at) {
        return -1;
    }
    let remaining = io.input.len().saturating_sub(io.read_at);
    let count = remaining.min(size as usize).min(io.read_chunk);
    if count != 0 {
        // SAFETY: AVS supplies a writable buffer of `size` bytes and `count <= size`.
        unsafe {
            ptr::copy_nonoverlapping(io.input.as_ptr().add(io.read_at), dst.cast(), count);
        }
        io.read_at += count;
    }
    count as c_int
}

unsafe extern "C" fn write_callback(_: c_int, src: *const c_void, size: c_int) -> c_int {
    if src.is_null() || size < 0 {
        return -1;
    }
    let mut io = IO.lock().unwrap();
    if io.write_fail_after.is_some_and(|at| io.output.len() >= at) {
        return -1;
    }
    let count = (size as usize).min(io.write_chunk);
    let bytes = unsafe { std::slice::from_raw_parts(src.cast::<u8>(), count) };
    io.output.extend_from_slice(bytes);
    count as c_int
}

unsafe fn symbol(module: Module, name: &str) -> Result<*mut c_void, String> {
    let name = CString::new(name).unwrap();
    let value = unsafe { GetProcAddress(module, name.as_ptr()) };
    if value.is_null() {
        Err(format!("missing export {}", name.to_string_lossy()))
    } else {
        Ok(value)
    }
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain([0]).collect()
}

fn number(value: Option<String>, default: i32) -> Result<i32, String> {
    let Some(value) = value else {
        return Ok(default);
    };
    if let Some(hex) = value.strip_prefix("0x") {
        i32::from_str_radix(hex, 16).map_err(|error| error.to_string())
    } else {
        value
            .parse()
            .map_err(|error: std::num::ParseIntError| error.to_string())
    }
}

fn usize_arg(value: Option<String>, default: usize) -> Result<usize, String> {
    value.map_or(Ok(default), |value| {
        value.parse::<usize>().map_err(|error| error.to_string())
    })
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let dll = args.next().ok_or("missing DLL path")?;
    let input = args.next().ok_or("missing input path")?;
    let output = args.next().ok_or("missing output path")?;
    let create_mode = number(args.next(), 0x4011)?;
    let read_type = number(args.next(), 0)?;
    let write_type = number(args.next(), 0)?;
    let output_mode = args
        .next()
        .map(|value| number(Some(value), 0))
        .transpose()?;
    let read_chunk = usize_arg(args.next(), usize::MAX)?;
    let write_chunk = usize_arg(args.next(), usize::MAX)?;
    let read_fail_after = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| error.to_string())?;
    let write_fail_after = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| error.to_string())?;
    let arena_mb = usize_arg(args.next(), 256)?;

    let module = unsafe { LoadLibraryW(wide(Path::new(&dll)).as_ptr()) };
    if module.is_null() {
        return Err("LoadLibraryW failed".into());
    }

    let result = (|| {
        let create: Create = unsafe { std::mem::transmute(symbol(module, "XCd229cc000126")?) };
        let destroy: Destroy = unsafe { std::mem::transmute(symbol(module, "XCd229cc00013c")?) };
        let file_read: FileRead = unsafe { std::mem::transmute(symbol(module, "XCd229cc00009a")?) };
        let file_write: FileWrite =
            unsafe { std::mem::transmute(symbol(module, "XCd229cc000024")?) };
        let mode: Mode = unsafe { std::mem::transmute(symbol(module, "XCd229cc000035")?) };

        let bytes = fs::read(&input).map_err(|error| error.to_string())?;
        {
            let mut io = IO.lock().unwrap();
            io.input = bytes;
            io.read_at = 0;
            io.output.clear();
            io.read_chunk = read_chunk;
            io.write_chunk = write_chunk;
            io.read_fail_after = read_fail_after;
            io.write_fail_after = write_fail_after;
        }

        // The property arena must be caller-owned and 8-byte aligned. A large
        // arena keeps this harness independent of AVS' private allocator.
        let arena_bytes = arena_mb * 1024 * 1024usize;
        let mut arena = vec![0u64; arena_bytes / 8];
        let property =
            unsafe { create(create_mode, arena.as_mut_ptr().cast(), arena_bytes as c_int) };
        if property.is_null() {
            return Err(format!(
                "avs_property_create failed for mode {create_mode:#x}"
            ));
        }

        let read_result = unsafe { file_read(property, read_type, read_callback, 0) };
        if read_result < 0 {
            unsafe { destroy(property) };
            return Err(format!("avs_property_file_read failed: {read_result:#x}"));
        }
        if let Some(output_mode) = output_mode {
            unsafe { mode(property as usize as c_int, output_mode) };
        }
        let write_result = unsafe { file_write(property, write_type, write_callback, 0) };
        unsafe { destroy(property) };
        if write_result < 0 {
            return Err(format!("avs_property_file_write failed: {write_result:#x}"));
        }

        let output_bytes = std::mem::take(&mut IO.lock().unwrap().output);
        fs::write(&output, &output_bytes).map_err(|error| error.to_string())?;
        println!(
            "read={read_result:#x} write={write_result:#x} bytes={}",
            output_bytes.len()
        );
        Ok(())
    })();

    unsafe { FreeLibrary(module) };
    result
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
