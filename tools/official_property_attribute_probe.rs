//! Probe official attribute traversal, conversion, and path behavior.

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
type Node = c_void;
type Create = unsafe extern "C" fn(c_int, *mut c_void, c_int) -> *mut Property;
type Destroy = unsafe extern "C" fn(*mut Property);
type FileRead = unsafe extern "C" fn(
    *mut Property,
    c_int,
    unsafe extern "C" fn(c_int, *mut c_void, c_int) -> c_int,
    c_int,
) -> c_int;
type Find = unsafe extern "C" fn(*mut Property, *mut Node, *const c_char) -> *mut Node;
type Traverse = unsafe extern "C" fn(*mut Node, c_int) -> *mut Node;
type Name = unsafe extern "C" fn(*mut Node, *mut c_char, c_int) -> c_int;
type PathFn = unsafe extern "C" fn(*mut Node, *mut c_char, c_int, u8) -> c_int;
type NodeInt = unsafe extern "C" fn(*mut Node) -> c_int;
type AttributeI32 = unsafe extern "C" fn(*mut Node, *mut i32) -> c_int;
type AttributeU32 = unsafe extern "C" fn(*mut Node, *mut u32) -> c_int;
type AttributeBool = unsafe extern "C" fn(*mut Node, *mut u8) -> c_int;
type Refer = unsafe extern "C" fn(
    *mut Property,
    *mut Node,
    *const c_char,
    c_int,
    *mut c_void,
    c_int,
) -> c_int;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> Module;
    fn GetProcAddress(module: Module, name: *const c_char) -> *mut c_void;
    fn FreeLibrary(module: Module) -> c_int;
}

static INPUT: Mutex<(Vec<u8>, usize)> = Mutex::new((Vec::new(), 0));

unsafe extern "C" fn read_callback(_: c_int, dst: *mut c_void, size: c_int) -> c_int {
    let mut input = INPUT.lock().unwrap();
    let count = input
        .0
        .len()
        .saturating_sub(input.1)
        .min(size.max(0) as usize);
    if count != 0 {
        unsafe { ptr::copy_nonoverlapping(input.0.as_ptr().add(input.1), dst.cast(), count) };
        input.1 += count;
    }
    count as c_int
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

fn cstr(value: &str) -> CString {
    CString::new(value).unwrap()
}

fn node_text(name: Name, node: *mut Node) -> String {
    if node.is_null() {
        return "NULL".into();
    }
    let mut out = [0u8; 128];
    unsafe { name(node, out.as_mut_ptr().cast(), out.len() as c_int) };
    let end = out.iter().position(|byte| *byte == 0).unwrap_or(out.len());
    String::from_utf8_lossy(&out[..end]).into_owned()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let dll = args.next().ok_or("usage: PROBE DLL XML")?;
    let xml = args.next().ok_or("usage: PROBE DLL XML")?;
    let attribute_mode = args
        .next()
        .map(|value| value.parse::<i32>().map_err(|error| error.to_string()))
        .transpose()?;
    INPUT.lock().unwrap().0 = fs::read(xml).map_err(|error| error.to_string())?;
    let module = unsafe { LoadLibraryW(wide(Path::new(&dll)).as_ptr()) };
    if module.is_null() {
        return Err("LoadLibraryW failed".into());
    }
    let result = (|| {
        macro_rules! load {
            ($name:literal, $ty:ty) => {
                unsafe { std::mem::transmute::<*mut c_void, $ty>(symbol(module, $name)?) }
            };
        }
        let create = load!("XCd229cc000126", Create);
        let destroy = load!("XCd229cc00013c", Destroy);
        let file_read = load!("XCd229cc00009a", FileRead);
        let find = load!("XCd229cc00012e", Find);
        let traverse = load!("XCd229cc000046", Traverse);
        let name = load!("XCd229cc000049", Name);
        let path = load!("XCd229cc00007c", PathFn);
        let node_type = load!("XCd229cc000071", NodeInt);
        let get_i32 = load!("XCd229cc00011a", AttributeI32);
        let get_u32 = load!("XCd229cc0000db", AttributeU32);
        let get_bool = load!("XCd229cc000110", AttributeBool);
        let refer = load!("XCd229cc000009", Refer);

        let mut arena = vec![0u64; 8 * 1024 * 1024 / 8];
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
        let status = unsafe { file_read(property, 0, read_callback, 0) };
        println!("read={status:#x}");
        if status < 0 {
            unsafe { destroy(property) };
            return Ok(());
        }
        let root = unsafe { find(property, ptr::null_mut(), cstr("/root").as_ptr()) };
        if let Some(mode) = attribute_mode {
            let attr = unsafe { traverse(root, 2) };
            let result = unsafe { traverse(attr, mode) };
            println!(
                "attribute traverse={mode} node={} type={:#x}",
                node_text(name, result),
                if result.is_null() {
                    -1
                } else {
                    unsafe { node_type(result) }
                }
            );
            unsafe { destroy(property) };
            return Ok(());
        }
        for candidate in ["b@/root", "s@/root", "text@/root", "/root/first", "first"] {
            let found = unsafe { find(property, ptr::null_mut(), cstr(candidate).as_ptr()) };
            println!("find {candidate:?}={}", node_text(name, found));
        }
        for mode in [0, 1, 2, 4, 5, 6, 7, 8] {
            let node = unsafe { traverse(root, mode) };
            println!(
                "root traverse={mode} node={} type={:#x}",
                node_text(name, node),
                if node.is_null() {
                    -1
                } else {
                    unsafe { node_type(node) }
                }
            );
        }
        let mut attr = unsafe { traverse(root, 2) };
        for index in 0..16 {
            if attr.is_null() {
                break;
            }
            let mut p0 = [0u8; 256];
            let mut p1 = [0u8; 256];
            let r0 = unsafe { path(attr, p0.as_mut_ptr().cast(), p0.len() as c_int, 0) };
            let r1 = unsafe { path(attr, p1.as_mut_ptr().cast(), p1.len() as c_int, 1) };
            let mut i = 0x5555_5555i32;
            let mut u = 0x5555_5555u32;
            let mut b = 0x55u8;
            println!(
                "attr[{index}] name={} type={:#x} path0={r0:#x}:{} path1={r1:#x}:{} i32={:#x}/{i} u32={:#x}/{u} bool={:#x}/{b}",
                node_text(name, attr),
                unsafe { node_type(attr) },
                node_text_from(&p0),
                node_text_from(&p1),
                unsafe { get_i32(attr, &mut i) },
                unsafe { get_u32(attr, &mut u) },
                unsafe { get_bool(attr, &mut b) },
            );
            attr = unsafe { traverse(attr, 4) };
        }
        for (parent, key) in [
            (root, "s"),
            (root, "first"),
            (root, "s@"),
            (ptr::null_mut(), "s@/root"),
            (ptr::null_mut(), "b@/root"),
            (ptr::null_mut(), "/root/first"),
        ] {
            for ty in [6, 7, 11, 52] {
                let mut out = [0x55u8; 64];
                let status = unsafe {
                    refer(
                        property,
                        parent,
                        cstr(key).as_ptr(),
                        ty,
                        out.as_mut_ptr().cast(),
                        out.len() as c_int,
                    )
                };
                println!(
                    "refer parent={} key={key:?} type={ty:#x} status={status:#x} raw={}",
                    !parent.is_null(),
                    hex(&out[..16])
                );
            }
        }
        unsafe { destroy(property) };
        Ok(())
    })();
    unsafe { FreeLibrary(module) };
    result
}

fn node_text_from(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
