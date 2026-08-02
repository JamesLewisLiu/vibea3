//! Behavioral probe for the actual property exports in libavs-win32.dll.

use std::{
    env,
    ffi::{c_char, c_int, c_void, CString},
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
type NodeCreate = unsafe extern "C" fn(
    *mut Property,
    *mut Node,
    c_int,
    *const c_char,
    *const c_void,
) -> *mut Node;
type NodeFind = unsafe extern "C" fn(*mut Property, *mut Node, *const c_char) -> *mut Node;
type NodeName = unsafe extern "C" fn(*mut Node, *mut c_char, c_int) -> c_int;
type NodePath = unsafe extern "C" fn(*mut Node, *mut c_void, c_int, u8) -> c_int;
type NodeType = unsafe extern "C" fn(*mut Node) -> c_int;
type NodeBool = unsafe extern "C" fn(*mut Node) -> c_int;
type NodeDataSize = unsafe extern "C" fn(*mut Node) -> c_int;
type NodeRefData = unsafe extern "C" fn(*mut Node) -> *mut c_void;
type NodeRead = unsafe extern "C" fn(*mut Node, c_int, *mut c_void, c_int) -> c_int;
type NodeWrite = unsafe extern "C" fn(*mut Node, c_int, *const c_char) -> c_int;
type NodeRename = unsafe extern "C" fn(*mut Node, *const c_char) -> c_int;
type NodeRemove = unsafe extern "C" fn(*mut Node) -> c_int;
type NodeTraverse = unsafe extern "C" fn(*mut Node, c_int) -> *mut Node;
type NodeQuery = unsafe extern "C" fn(*mut Property, *mut Node, *mut c_int) -> c_int;
type PropertyInt = unsafe extern "C" fn(*mut Property) -> c_int;
type PropertyFree = unsafe extern "C" fn(*mut Property, *mut c_int) -> c_int;
type PropertyToken = unsafe extern "C" fn(*mut Property, c_int) -> c_int;
type NodeFingerprint = unsafe extern "C" fn(*mut Property, *mut Node, *mut c_void, c_int) -> c_int;
type NodeClone = unsafe extern "C" fn(*mut Property, *mut Node, *mut Node, u8) -> *mut Node;
type NodeGetDesc = unsafe extern "C" fn(*mut Node) -> *mut Property;
type NodeHas = unsafe extern "C" fn(*mut Node, c_int) -> c_int;
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

static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

unsafe extern "C" fn write_callback(_: c_int, src: *const c_void, size: c_int) -> c_int {
    if src.is_null() || size < 0 {
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(src.cast::<u8>(), size as usize) };
    OUTPUT.lock().unwrap().extend_from_slice(bytes);
    size
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

fn display_bytes(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn main() -> Result<(), String> {
    let fingerprint_probe = env::args().any(|arg| arg == "--fingerprint");
    let dll = env::args()
        .nth(1)
        .ok_or("usage: official_property_probe DLL")?;
    let traversal_probe = env::args()
        .nth(2)
        .map(|value| value.parse::<i32>().map_err(|error| error.to_string()))
        .transpose()?;
    let mode_probe = env::args()
        .nth(3)
        .map(|value| {
            value
                .strip_prefix("0x")
                .map_or_else(|| value.parse::<i32>(), |hex| i32::from_str_radix(hex, 16))
                .map_err(|error| error.to_string())
        })
        .transpose()?;
    let traversal_start = env::args().nth(4).unwrap_or_else(|| "root".into());
    let module = unsafe { LoadLibraryW(wide(Path::new(&dll)).as_ptr()) };
    if module.is_null() {
        return Err("LoadLibraryW failed".into());
    }

    let result = (|| {
        macro_rules! load {
            ($name:literal, $ty:ty) => {{
                unsafe { std::mem::transmute::<*mut c_void, $ty>(symbol(module, $name)?) }
            }};
        }

        let create = load!("XCd229cc000126", Create);
        let destroy = load!("XCd229cc00013c", Destroy);
        let node_create = load!("XCd229cc00002c", NodeCreate);
        let node_find = load!("XCd229cc00012e", NodeFind);
        let node_name = load!("XCd229cc000049", NodeName);
        let node_path = load!("XCd229cc00007c", NodePath);
        let node_type = load!("XCd229cc000071", NodeType);
        let node_is_array = load!("XCd229cc000142", NodeBool);
        let node_datasize = load!("XCd229cc000083", NodeDataSize);
        let node_refdata = load!("XCd229cc00009f", NodeRefData);
        let node_read = load!("XCd229cc0000f3", NodeRead);
        let node_write = load!("XCd229cc00002d", NodeWrite);
        let node_rename = load!("XCd229cc0000af", NodeRename);
        let node_remove = load!("XCd229cc000028", NodeRemove);
        let node_traverse = load!("XCd229cc000046", NodeTraverse);
        let node_query = load!("XCd229cc0000b1", NodeQuery);
        let query_size = load!("XCd229cc000032", PropertyInt);
        let query_free = load!("XCd229cc000144", PropertyFree);
        let get_error = load!("XCd229cc0000b5", PropertyInt);
        let clear_error = load!("XCd229cc00014b", PropertyInt);
        let lock = load!("XCd229cc000121", PropertyToken);
        let unlock = load!("XCd229cc000145", PropertyToken);
        let fingerprint = load!("XCd229cc000057", NodeFingerprint);
        let node_clone = load!("XCd229cc00010a", NodeClone);
        let node_get_desc = load!("XCd229cc000165", NodeGetDesc);
        let node_has = load!("XCd229cc00008a", NodeHas);
        let file_write = load!("XCd229cc000024", FileWrite);
        let mode = load!("XCd229cc000035", Mode);

        let mut arena = vec![0u64; 8 * 1024 * 1024 / 8];
        let property = unsafe { create(7, arena.as_mut_ptr().cast(), (arena.len() * 8) as c_int) };
        if property.is_null() {
            return Err("create(mode=7) failed".into());
        }

        let root = unsafe {
            node_create(
                property,
                ptr::null_mut(),
                1,
                cstr("/root").as_ptr(),
                ptr::null(),
            )
        };
        let mut free_detail = 0x5555_5555;
        println!(
            "initial size={:#x} free={:#x} free_detail={:#x} error={:#x}",
            unsafe { query_size(property) },
            unsafe { query_free(property, &mut free_detail) },
            free_detail,
            unsafe { get_error(property) },
        );
        println!("lock token=0 result={:#x}", unsafe { lock(property, 0) });
        println!("lock token=0x1234 result={:#x}", unsafe {
            lock(property, 0x1234)
        });
        let locked_create = unsafe {
            node_create(
                property,
                ptr::null_mut(),
                1,
                cstr("/root/locked").as_ptr(),
                ptr::null(),
            )
        };
        println!("create while locked={}", !locked_create.is_null());
        for token in [0x1234, 0x5678] {
            println!("lock token={token:#x} result={:#x}", unsafe {
                lock(property, token)
            });
        }
        for token in [0x5678, 0x1234, 0x1234, 0] {
            println!("unlock token={token:#x} result={:#x}", unsafe {
                unlock(property, token)
            });
        }
        let text_value = cstr("hello & goodbye");
        let _text = unsafe {
            node_create(
                property,
                ptr::null_mut(),
                11,
                cstr("/root/text").as_ptr(),
                text_value.as_ptr().cast(),
            )
        };
        let number_value = -123_456i32;
        let _number = unsafe {
            node_create(
                property,
                ptr::null_mut(),
                6,
                cstr("/root/number").as_ptr(),
                number_value as usize as *const c_void,
            )
        };
        let attr_value = cstr("42");
        let mut attr: *mut Node = ptr::null_mut();
        for (parent, ty, path) in [
            (ptr::null_mut(), 11, "answer@/root"),
            (root, 11, "answer@"),
            (root, 11, "@answer"),
            (root, 0x2e, "answer"),
        ] {
            if attr.is_null() {
                attr = unsafe {
                    node_create(
                        property,
                        parent,
                        ty,
                        cstr(path).as_ptr(),
                        attr_value.as_ptr().cast(),
                    )
                };
                println!(
                    "create attr parent={} type={ty:#x} path={path:?} success={}",
                    !parent.is_null(),
                    !attr.is_null()
                );
            }
        }
        // Any successful mutation may compact the arena and invalidate every
        // previously returned node pointer, so reacquire all handles.
        let root = unsafe { node_find(property, ptr::null_mut(), cstr("/root").as_ptr()) };
        let text = unsafe { node_find(property, ptr::null_mut(), cstr("/root/text").as_ptr()) };
        let number = unsafe { node_find(property, ptr::null_mut(), cstr("/root/number").as_ptr()) };
        println!(
            "create root={} text={} number={} attr={}",
            !root.is_null(),
            !text.is_null(),
            !number.is_null(),
            !attr.is_null()
        );
        println!(
            "root desc_matches={} has={:?}",
            unsafe { node_get_desc(root) } == property,
            (0..=4)
                .map(|has_mode| unsafe { node_has(root, has_mode) })
                .collect::<Vec<_>>()
        );
        if fingerprint_probe {
            let mut root_fingerprint = vec![0x55u8; 4096];
            println!(
                "root fingerprint result={:#x} bytes={}",
                unsafe {
                    fingerprint(
                        property,
                        root,
                        root_fingerprint.as_mut_ptr().cast(),
                        root_fingerprint.len() as c_int,
                    )
                },
                hex(&root_fingerprint[..32]),
            );
        }

        for path in [
            "",
            "/",
            "root",
            "/root",
            "/root/text",
            "root/text",
            "answer@/root",
            "/missing",
        ] {
            let found = unsafe { node_find(property, ptr::null_mut(), cstr(path).as_ptr()) };
            println!("find {path:?}={}", !found.is_null());
        }

        for (label, node) in [
            ("root", root),
            ("text", text),
            ("number", number),
            ("attr", attr),
        ] {
            if node.is_null() {
                continue;
            }
            let mut name = [0x55u8; 64];
            let name_result =
                unsafe { node_name(node, name.as_mut_ptr().cast(), name.len() as c_int) };
            let mut path = [0x55u8; 256];
            let path_result =
                unsafe { node_path(node, path.as_mut_ptr().cast(), path.len() as c_int, 0) };
            let mut stat = [0x5555_5555i32; 16];
            let stat_result = unsafe { node_query(property, node, stat.as_mut_ptr()) };
            println!(
                "node {label} name_result={name_result:#x} name={:?} path_result={path_result:#x} path={:?} type={:#x} array={} datasize={} ref={} stat_result={stat_result:#x} stat={:?}",
                display_bytes(&name),
                display_bytes(&path),
                unsafe { node_type(node) },
                unsafe { node_is_array(node) },
                unsafe { node_datasize(node) },
                !unsafe { node_refdata(node) }.is_null(),
                &stat[..8],
            );
        }

        if !text.is_null() {
            for size in [0, 1, 4, 5, 8] {
                let mut out = [0x55u8; 16];
                let result = unsafe { node_name(text, out.as_mut_ptr().cast(), size) };
                println!(
                    "name text size={size} result={result:#x} out={:?}",
                    display_bytes(&out)
                );
            }
            for size in [0, 1, 2, 5, 16, 64] {
                let mut out = [0x55u8; 64];
                let result = unsafe { node_read(text, 11, out.as_mut_ptr().cast(), size) };
                println!(
                    "read text size={size} result={result:#x} out={:?}",
                    display_bytes(&out)
                );
            }
            for ty in [2, 3, 4, 5, 6, 7, 8, 9, 11, 14, 15, 52] {
                let mut out = [0x55u8; 64];
                let result =
                    unsafe { node_read(text, ty, out.as_mut_ptr().cast(), out.len() as c_int) };
                println!(
                    "coerce text type={ty:#x} result={result:#x} raw={}",
                    hex(&out[..16])
                );
            }
            let changed = cstr("changed");
            println!("write text={:#x}", unsafe {
                node_write(text, 11, changed.as_ptr())
            });
            let text = unsafe { node_find(property, ptr::null_mut(), cstr("/root/text").as_ptr()) };
            println!("rename text={:#x}", unsafe {
                node_rename(text, cstr("renamed").as_ptr())
            });
        }

        let number = unsafe { node_find(property, ptr::null_mut(), cstr("/root/number").as_ptr()) };
        if !number.is_null() {
            for ty in [2, 3, 4, 5, 6, 7, 8, 9, 11, 14, 15, 52] {
                let mut out = [0x55u8; 64];
                let result =
                    unsafe { node_read(number, ty, out.as_mut_ptr().cast(), out.len() as c_int) };
                println!(
                    "coerce number type={ty:#x} result={result:#x} raw={}",
                    hex(&out[..16])
                );
            }
        }

        let copy = unsafe {
            node_create(
                property,
                ptr::null_mut(),
                1,
                cstr("/root/copy").as_ptr(),
                ptr::null(),
            )
        };
        let text = unsafe { node_find(property, ptr::null_mut(), cstr("/root/renamed").as_ptr()) };
        if !copy.is_null() && !text.is_null() {
            let cloned = unsafe { node_clone(property, copy, text, 1) };
            println!("clone recursive success={}", !cloned.is_null());
        }

        println!("sticky error before clear={:#x}", unsafe {
            get_error(property)
        });
        println!("clear error result={:#x}", unsafe { clear_error(property) });
        println!("sticky error after clear={:#x}", unsafe {
            get_error(property)
        });

        let traversal_path = format!("/root/{}", traversal_start.trim_start_matches("root/"));
        let traversal_node = if traversal_start == "root" {
            unsafe { node_find(property, ptr::null_mut(), cstr("/root").as_ptr()) }
        } else {
            unsafe { node_find(property, ptr::null_mut(), cstr(&traversal_path).as_ptr()) }
        };
        if !traversal_node.is_null() {
            let traversal_modes: Vec<i32> =
                traversal_probe.map_or_else(|| vec![0, 1, 2], |mode| vec![mode]);
            for traversal_mode in traversal_modes {
                let next = unsafe { node_traverse(traversal_node, traversal_mode) };
                let mut name = [0u8; 64];
                if !next.is_null() {
                    unsafe { node_name(next, name.as_mut_ptr().cast(), name.len() as c_int) };
                }
                println!(
                    "traverse mode={traversal_mode} found={} name={:?}",
                    !next.is_null(),
                    display_bytes(&name)
                );
            }
        }

        let number = unsafe { node_find(property, ptr::null_mut(), cstr("/root/number").as_ptr()) };
        if !number.is_null() {
            println!("remove number={:#x}", unsafe { node_remove(number) });
        }

        if let Some(output_mode) = mode_probe {
            unsafe { mode(property as usize as c_int, output_mode) };
            OUTPUT.lock().unwrap().clear();
            let result = unsafe { file_write(property, 0, write_callback, 0) };
            let output = OUTPUT.lock().unwrap();
            println!(
                "mode={output_mode:#x} result={result:#x} len={} head={}",
                output.len(),
                hex(&output[..output.len().min(12)])
            );
        } else {
            OUTPUT.lock().unwrap().clear();
            let write_result = unsafe { file_write(property, 0, write_callback, 0) };
            let xml = String::from_utf8_lossy(&OUTPUT.lock().unwrap()).into_owned();
            println!("xml write={write_result:#x} {xml}");
        }

        unsafe { destroy(property) };
        Ok(())
    })();

    unsafe { FreeLibrary(module) };
    result
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
