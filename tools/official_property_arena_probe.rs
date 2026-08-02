//! Probes avs_property_create arena sizing, alignment, and mode acceptance.

use std::{
    env,
    ffi::{CString, c_char, c_int, c_void},
    os::windows::ffi::OsStrExt,
    path::Path,
};

type Module = *mut c_void;
type Property = c_void;
type Create = unsafe extern "C" fn(c_int, *mut c_void, c_int) -> *mut Property;
type Destroy = unsafe extern "C" fn(*mut Property);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> Module;
    fn GetProcAddress(module: Module, name: *const c_char) -> *mut c_void;
    fn FreeLibrary(module: Module) -> c_int;
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
    let dll = env::args().nth(1).ok_or("usage: official_property_arena_probe DLL")?;
    let module = unsafe { LoadLibraryW(wide(Path::new(&dll)).as_ptr()) };
    if module.is_null() {
        return Err("LoadLibraryW failed".into());
    }
    let create: Create = unsafe { std::mem::transmute(symbol(module, "XCd229cc000126")?) };
    let destroy: Destroy = unsafe { std::mem::transmute(symbol(module, "XCd229cc00013c")?) };

    for mode in [6, 7, 0x11, 0x1007, 0x1011, 0x4007, 0x4011] {
        for size in [
            575usize, 576, 577, 578, 583, 584, 631, 632, 633, 634, 635, 636, 637, 638,
            639, 640, 847, 848, 871, 872, 1024,
        ] {
            for offset in 0..8usize {
                let mut arena = vec![0u64; (size + offset + 15).div_ceil(8)];
                let base = arena.as_mut_ptr().cast::<u8>();
                let input = unsafe { base.add(offset) };
                let property = unsafe { create(mode, input.cast(), size as c_int) };
                let relative = (!property.is_null()).then(|| property as usize - base as usize);
                println!(
                    "mode={mode:#x} size={size} offset={offset} success={} relative={relative:?}",
                    !property.is_null()
                );
                if !property.is_null() {
                    unsafe { destroy(property) };
                }
            }
        }
    }

    unsafe { FreeLibrary(module) };
    Ok(())
}
