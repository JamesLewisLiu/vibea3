//! Probe the official property's sizing and contiguous-memory writer APIs.
//!
//! Build with:
//! `rustc --target i686-pc-windows-msvc -O tools/official_property_memory_probe.rs`

use std::{
    env,
    ffi::{c_char, c_int, c_void, CString},
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
type ReadCalc = unsafe extern "C" fn(
    unsafe extern "C" fn(c_int, *mut c_void, c_int) -> c_int,
    c_int,
    *mut u32,
    *mut *mut c_void,
) -> c_int;
type ReadCalcLong = unsafe extern "C" fn(
    unsafe extern "C" fn(c_int, *mut c_void, c_int) -> c_int,
    c_int,
    *mut u32,
    *mut u32,
    *mut u32,
) -> c_int;
type QuerySize = unsafe extern "C" fn(*mut Property) -> c_int;
type WriteMemory = unsafe extern "C" fn(*mut Property, *mut c_void, c_int) -> c_int;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> Module;
    fn GetProcAddress(module: Module, name: *const c_char) -> *mut c_void;
    fn FreeLibrary(module: Module) -> c_int;
}

#[derive(Default)]
struct Input {
    bytes: Vec<u8>,
    at: usize,
    chunk: usize,
}

static INPUT: Mutex<Input> = Mutex::new(Input {
    bytes: Vec::new(),
    at: 0,
    chunk: usize::MAX,
});

unsafe extern "C" fn read_callback(_: c_int, dst: *mut c_void, size: c_int) -> c_int {
    if dst.is_null() || size <= 0 {
        return 0;
    }
    let mut input = INPUT.lock().unwrap();
    let count = input
        .bytes
        .len()
        .saturating_sub(input.at)
        .min(size as usize)
        .min(input.chunk);
    if count != 0 {
        unsafe {
            ptr::copy_nonoverlapping(input.bytes.as_ptr().add(input.at), dst.cast(), count);
        }
        input.at += count;
    }
    count as c_int
}

fn reset(bytes: &[u8], chunk: usize) {
    let mut input = INPUT.lock().unwrap();
    input.bytes.clear();
    input.bytes.extend_from_slice(bytes);
    input.at = 0;
    input.chunk = chunk;
}

unsafe fn symbol(module: Module, name: &str) -> Result<*mut c_void, String> {
    let name = CString::new(name).unwrap();
    let value = unsafe { GetProcAddress(module, name.as_ptr()) };
    (!value.is_null())
        .then_some(value)
        .ok_or_else(|| format!("missing export {}", name.to_string_lossy()))
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain([0]).collect()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let dll = args.next().ok_or("usage: PROBE DLL INPUT")?;
    let input_path = args.next().ok_or("usage: PROBE DLL INPUT")?;
    let bytes = fs::read(input_path).map_err(|error| error.to_string())?;
    let module = unsafe { LoadLibraryW(wide(Path::new(&dll)).as_ptr()) };
    if module.is_null() {
        return Err("LoadLibraryW failed".into());
    }
    let result = (|| {
        let create: Create = unsafe { std::mem::transmute(symbol(module, "XCd229cc000126")?) };
        let destroy: Destroy = unsafe { std::mem::transmute(symbol(module, "XCd229cc00013c")?) };
        let file_read: FileRead = unsafe { std::mem::transmute(symbol(module, "XCd229cc00009a")?) };
        let calc: ReadCalc = unsafe { std::mem::transmute(symbol(module, "XCd229cc0000ff")?) };
        let calc_long: ReadCalcLong =
            unsafe { std::mem::transmute(symbol(module, "XCd229cc00002b")?) };
        let query_size: QuerySize =
            unsafe { std::mem::transmute(symbol(module, "XCd229cc000032")?) };
        let write_memory: WriteMemory =
            unsafe { std::mem::transmute(symbol(module, "XCd229cc000033")?) };

        for chunk in [1, 2, 7, usize::MAX] {
            for outputs in 0..4 {
                reset(&bytes, chunk);
                let mut size = 0xcccc_ccccu32;
                let mut buffer = 0xdddd_ddddu32 as *mut c_void;
                let size_ptr = (outputs & 1 != 0)
                    .then_some(&mut size as *mut u32)
                    .unwrap_or(ptr::null_mut());
                let buffer_ptr = (outputs & 2 != 0)
                    .then_some(&mut buffer as *mut *mut c_void)
                    .unwrap_or(ptr::null_mut());
                let status = unsafe { calc(read_callback, 0, size_ptr, buffer_ptr) };
                println!(
                    "calc chunk={chunk} outputs={outputs} status={status:#x} size={size:#x} buffer={buffer:p} consumed={}",
                    INPUT.lock().unwrap().at
                );
            }
            reset(&bytes, chunk);
            let mut nodes = 0xcccc_cccc;
            let mut names = 0xdddd_dddd;
            let mut data = 0xeeee_eeee;
            let status = unsafe { calc_long(read_callback, 0, &mut nodes, &mut names, &mut data) };
            println!(
                "calc_long chunk={chunk} status={status:#x} nodes={nodes:#x} names={names:#x} data={data:#x} consumed={}",
                INPUT.lock().unwrap().at
            );
        }

        let mut arena = vec![0u64; 32 * 1024 * 1024 / 8];
        let property = unsafe {
            create(
                0x4011,
                arena.as_mut_ptr().cast(),
                (arena.len() * 8) as c_int,
            )
        };
        if property.is_null() {
            return Err("property_create failed".into());
        }
        reset(&bytes, usize::MAX);
        let status = unsafe { file_read(property, 0, read_callback, 0) };
        println!("file_read status={status:#x}");
        if status < 0 {
            unsafe { destroy(property) };
            return Ok(());
        }

        let queried = unsafe { query_size(property) };
        println!("query_size status={queried:#x}");
        let mut exact = vec![0xccu8; bytes.len().saturating_mul(8).max(4096)];
        let required =
            unsafe { write_memory(property, exact.as_mut_ptr().cast(), exact.len() as c_int) };
        println!("write_memory large status={required:#x}");
        for size in [
            0,
            1,
            required.saturating_sub(1),
            required,
            required.saturating_add(1),
        ] {
            let alloc = (size.max(0) as usize).max(1);
            let mut output = vec![0xccu8; alloc + 16];
            let status = unsafe { write_memory(property, output.as_mut_ptr().cast(), size) };
            let changed = output.iter().position(|byte| *byte != 0xcc);
            println!("write_memory size={size} status={status:#x} first_changed={changed:?}");
        }
        let null_status = unsafe { write_memory(property, ptr::null_mut(), required) };
        println!("write_memory null status={null_status:#x}");
        unsafe { destroy(property) };
        Ok(())
    })();
    unsafe { FreeLibrary(module) };
    result
}
