//! VST3 binary probing: load a .vst3 bundle and extract parameter metadata
//! (names, units, defaults) directly from the VST3 COM interfaces, without
//! needing a running SuperCollider instance.

use std::ffi::{c_char, c_void};
use std::path::Path;

use libloading::{Library, Symbol};
use vst3::ComPtr;
use vst3::Steinberg::{
    kResultOk, IPluginFactory, IPluginFactoryTrait, IPluginBaseTrait,
    PClassInfo,
};
use vst3::Steinberg::Vst::{
    IComponent, IComponentTrait, IEditController, IEditControllerTrait,
    ParameterInfo,
};
use vst3::Interface;

/// Discovered parameter metadata from a VST3 plugin
#[derive(Debug, Clone)]
pub struct Vst3ParamInfo {
    pub index: i32,
    pub id: u32,
    pub name: String,
    pub units: String,
    pub default_normalized: f64,
    pub step_count: i32,
    pub flags: i32,
}

/// Convert a VST3 String128 (UTF-16, null-terminated) to a Rust String.
/// String128 is `[u16; 128]` in the vst3 crate.
fn string128_to_string(s: &[u16; 128]) -> String {
    let len = s.iter().position(|&c| c == 0).unwrap_or(128);
    String::from_utf16_lossy(&s[..len])
}

/// Convert a char8 array to a String (null-terminated C string bytes)
fn char8_array_to_string(s: &[c_char]) -> String {
    let len = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    let bytes: Vec<u8> = s[..len].iter().map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Resolve the actual binary path inside a .vst3 bundle (platform-specific).
///
/// Bundle layouts:
/// - macOS: `Plugin.vst3/Contents/MacOS/<binary>`
/// - Linux: `Plugin.vst3/Contents/x86_64-linux/<name>.so` or `aarch64-linux/`
/// - Windows: `Plugin.vst3/Contents/x86_64-win/<name>.vst3`
fn resolve_vst3_binary(bundle_path: &Path) -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        resolve_vst3_binary_macos(bundle_path)
    }

    #[cfg(target_os = "linux")]
    {
        resolve_vst3_binary_linux(bundle_path)
    }

    #[cfg(target_os = "windows")]
    {
        resolve_vst3_binary_windows(bundle_path)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(format!(
            "VST3 probing not supported on this platform: {}",
            bundle_path.display()
        ))
    }
}

#[cfg(target_os = "macos")]
fn resolve_vst3_binary_macos(bundle_path: &Path) -> Result<std::path::PathBuf, String> {
    let macos_dir = bundle_path.join("Contents").join("MacOS");
    if !macos_dir.is_dir() {
        return Err(format!("No Contents/MacOS directory in {}", bundle_path.display()));
    }

    // Try the stem name first (most common convention)
    let stem = bundle_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let stem_path = macos_dir.join(stem);
    if stem_path.is_file() {
        return Ok(stem_path);
    }

    // Fallback: pick the first file in Contents/MacOS/
    find_first_file_in_dir(&macos_dir)
}

#[cfg(target_os = "linux")]
fn resolve_vst3_binary_linux(bundle_path: &Path) -> Result<std::path::PathBuf, String> {
    // Linux VST3 bundle structure: Contents/<arch>/*.so
    // Try x86_64-linux first (most common), then aarch64-linux for ARM
    let arch_dirs = ["x86_64-linux", "aarch64-linux"];

    for arch in arch_dirs {
        let arch_dir = bundle_path.join("Contents").join(arch);
        if arch_dir.is_dir() {
            // Look for .so file
            if let Ok(entries) = std::fs::read_dir(&arch_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if ext == "so" {
                                return Ok(path);
                            }
                        }
                    }
                }
            }
        }
    }

    Err(format!(
        "No Linux binary found in VST3 bundle (checked x86_64-linux, aarch64-linux): {}",
        bundle_path.display()
    ))
}

#[cfg(target_os = "windows")]
fn resolve_vst3_binary_windows(bundle_path: &Path) -> Result<std::path::PathBuf, String> {
    // Windows VST3 bundle structure: Contents/x86_64-win/*.vst3
    let arch_dirs = ["x86_64-win", "arm64-win"];

    for arch in arch_dirs {
        let arch_dir = bundle_path.join("Contents").join(arch);
        if arch_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&arch_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if ext == "vst3" {
                                return Ok(path);
                            }
                        }
                    }
                }
            }
        }
    }

    Err(format!(
        "No Windows binary found in VST3 bundle: {}",
        bundle_path.display()
    ))
}

/// Find the first file in a directory (helper for bundle resolution)
#[cfg(target_os = "macos")]
fn find_first_file_in_dir(dir: &Path) -> Result<std::path::PathBuf, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read {}: {}", dir.display(), e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(format!("No binary found in {}", dir.display()))
}

/// Return platform-specific VST3 search directories.
pub fn vst3_search_paths() -> Vec<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Audio/Plug-Ins/VST3"));
        }
        paths.push(std::path::PathBuf::from("/Library/Audio/Plug-Ins/VST3"));
        paths
    }

    #[cfg(target_os = "linux")]
    {
        let mut paths = Vec::new();
        // User directory first (highest priority)
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".vst3"));
        }
        // System directories
        paths.push(std::path::PathBuf::from("/usr/lib/vst3"));
        paths.push(std::path::PathBuf::from("/usr/local/lib/vst3"));
        paths
    }

    #[cfg(target_os = "windows")]
    {
        let mut paths = Vec::new();
        // Common Files location
        if let Some(program_files) = std::env::var_os("CommonProgramFiles") {
            paths.push(std::path::PathBuf::from(program_files).join("VST3"));
        }
        paths
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

/// Probe a VST3 bundle for parameter metadata.
///
/// Loads the VST3 binary, instantiates the component and controller via COM,
/// queries all parameters, then cleans up. Returns parameter info or an error.
///
/// The entire operation is wrapped in `catch_unwind` to handle plugin crashes.
pub fn probe_vst3_params(bundle_path: &Path) -> Result<Vec<Vst3ParamInfo>, String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        probe_vst3_params_inner(bundle_path)
    })) {
        Ok(result) => result,
        Err(_) => Err(format!(
            "VST3 plugin panicked during probing: {}",
            bundle_path.display()
        )),
    }
}

/// RAII guard that releases a Core Foundation `CFBundleRef` on drop.
/// Used to ensure the CFBundle passed to `bundleEntry` is cleaned up on all paths.
#[cfg(target_os = "macos")]
struct CfBundleGuard(*mut c_void);

#[cfg(target_os = "macos")]
impl Drop for CfBundleGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            extern "C" {
                fn CFRelease(cf: *const c_void);
            }
            unsafe { CFRelease(self.0 as *const _); }
        }
    }
}

fn probe_vst3_params_inner(bundle_path: &Path) -> Result<Vec<Vst3ParamInfo>, String> {
    let binary_path = resolve_vst3_binary(bundle_path)?;

    // Load the dynamic library
    let lib = unsafe { Library::new(&binary_path) }
        .map_err(|e| format!("Failed to load {}: {}", binary_path.display(), e))?;

    // Platform-specific module initialization.
    //
    // macOS: Call bundleEntry with a CFBundleRef before GetPluginFactory.
    // The VST3 SDK's default bundleEntry implementation calls CFRetain on its
    // argument, so passing garbage (or nothing) causes a segfault.
    //
    // Linux: Call ModuleEntry (if present). The VST3 SDK on Linux uses
    // ModuleEntry/ModuleExit instead of bundleEntry/bundleExit.

    #[cfg(target_os = "macos")]
    let _cf_bundle_guard = {
        extern "C" {
            fn CFURLCreateFromFileSystemRepresentation(
                allocator: *const c_void,
                buffer: *const u8,
                buf_len: isize,
                is_directory: u8,
            ) -> *mut c_void;
            fn CFBundleCreate(
                allocator: *const c_void,
                bundle_url: *const c_void,
            ) -> *mut c_void;
            fn CFRelease(cf: *const c_void);
        }

        let path_bytes = bundle_path.as_os_str().as_encoded_bytes();
        let url = unsafe {
            CFURLCreateFromFileSystemRepresentation(
                std::ptr::null(),
                path_bytes.as_ptr(),
                path_bytes.len() as isize,
                1,
            )
        };
        let cf_bundle = if !url.is_null() {
            let b = unsafe { CFBundleCreate(std::ptr::null(), url) };
            unsafe { CFRelease(url as *const _); }
            b
        } else {
            std::ptr::null_mut()
        };

        type BundleEntryFn = unsafe extern "C" fn(bundle: *mut c_void) -> bool;
        if let Ok(bundle_entry) = unsafe { lib.get::<BundleEntryFn>(b"bundleEntry") } {
            unsafe { bundle_entry(cf_bundle); }
        }

        CfBundleGuard(cf_bundle)
    };

    #[cfg(target_os = "linux")]
    let _module_entry_called = {
        // Linux VST3 plugins use ModuleEntry instead of bundleEntry.
        // The function signature is: bool ModuleEntry(void* sharedLibraryHandle)
        // We pass null as the handle - most plugins don't actually use it.
        type ModuleEntryFn = unsafe extern "C" fn(handle: *mut c_void) -> bool;
        if let Ok(module_entry) = unsafe { lib.get::<ModuleEntryFn>(b"ModuleEntry") } {
            unsafe { module_entry(std::ptr::null_mut()) };
            true
        } else {
            false
        }
    };

    // Get the plugin factory
    type GetFactoryFn = unsafe extern "system" fn() -> *mut c_void;
    let get_factory: Symbol<GetFactoryFn> = unsafe { lib.get(b"GetPluginFactory") }
        .map_err(|e| format!("No GetPluginFactory symbol: {}", e))?;
    let factory_raw = unsafe { get_factory() };
    if factory_raw.is_null() {
        cleanup_bundle_exit(&lib);
        return Err("GetPluginFactory returned null".into());
    }

    let factory: ComPtr<IPluginFactory> =
        unsafe { ComPtr::from_raw(factory_raw as *mut IPluginFactory) }
            .ok_or("Failed to wrap factory pointer")?;

    // Find the audio processor class
    let class_count = unsafe { factory.countClasses() };
    let mut target_cid = None;

    for i in 0..class_count {
        let mut info: PClassInfo = unsafe { std::mem::zeroed() };
        let result = unsafe { factory.getClassInfo(i, &mut info) };
        if result != kResultOk {
            continue;
        }
        let category = char8_array_to_string(&info.category);
        if category.starts_with("Audio Module Class") {
            target_cid = Some(info.cid);
            break;
        }
    }

    let target_cid = match target_cid {
        Some(cid) => cid,
        None => {
            drop(factory);
            cleanup_bundle_exit(&lib);
            return Err("No Audio Module Class found in plugin".into());
        }
    };

    // Create the component instance
    let mut component_raw: *mut c_void = std::ptr::null_mut();
    let icomponent_iid = IComponent::IID;
    let result = unsafe {
        factory.createInstance(
            target_cid.as_ptr() as *const c_char,
            icomponent_iid.as_ptr() as *const c_char,
            &mut component_raw,
        )
    };
    if result != kResultOk || component_raw.is_null() {
        drop(factory);
        cleanup_bundle_exit(&lib);
        return Err("Failed to create IComponent instance".into());
    }
    let component: ComPtr<IComponent> =
        unsafe { ComPtr::from_raw(component_raw as *mut IComponent) }
            .ok_or("Failed to wrap component pointer")?;

    // Initialize the component (pass null host context)
    let _ = unsafe { component.initialize(std::ptr::null_mut()) };

    // Try to get IEditController: first via QueryInterface on component (single-component plugins)
    let controller: Option<ComPtr<IEditController>> = component.cast();

    let controller = if let Some(ctrl) = controller {
        ctrl
    } else {
        // Get the controller class ID and create it separately
        let mut controller_cid = [0i8; 16];
        let result = unsafe { component.getControllerClassId(&mut controller_cid) };
        if result != kResultOk {
            let _ = unsafe { component.terminate() };
            drop(component);
            drop(factory);
            cleanup_bundle_exit(&lib);
            return Err("Could not get controller class ID".into());
        }

        let mut ctrl_raw: *mut c_void = std::ptr::null_mut();
        let ieditcontroller_iid = IEditController::IID;
        let result = unsafe {
            factory.createInstance(
                controller_cid.as_ptr() as *const c_char,
                ieditcontroller_iid.as_ptr() as *const c_char,
                &mut ctrl_raw,
            )
        };
        if result != kResultOk || ctrl_raw.is_null() {
            let _ = unsafe { component.terminate() };
            drop(component);
            drop(factory);
            cleanup_bundle_exit(&lib);
            return Err("Failed to create IEditController instance".into());
        }
        let ctrl: ComPtr<IEditController> =
            unsafe { ComPtr::from_raw(ctrl_raw as *mut IEditController) }
                .ok_or_else(|| {
                    let _ = unsafe { component.terminate() };
                    "Failed to wrap controller pointer".to_string()
                })?;
        // Initialize the controller
        let _ = unsafe { ctrl.initialize(std::ptr::null_mut()) };
        ctrl
    };

    // Query parameters
    let param_count = unsafe { controller.getParameterCount() };
    let mut params = Vec::with_capacity(param_count.max(0) as usize);

    for i in 0..param_count {
        let mut info: ParameterInfo = unsafe { std::mem::zeroed() };
        let result = unsafe { controller.getParameterInfo(i, &mut info) };
        if result != kResultOk {
            continue;
        }
        params.push(Vst3ParamInfo {
            index: i,
            id: info.id,
            name: string128_to_string(&info.title),
            units: string128_to_string(&info.units),
            default_normalized: info.defaultNormalizedValue,
            step_count: info.stepCount,
            flags: info.flags,
        });
    }

    // Cleanup
    let _ = unsafe { controller.terminate() };
    let _ = unsafe { component.terminate() };
    drop(controller);
    drop(component);
    drop(factory);
    cleanup_bundle_exit(&lib);
    drop(lib);

    Ok(params)
}

/// Call bundleExit (macOS) or ModuleExit (Linux) to clean up the plugin module.
fn cleanup_bundle_exit(_lib: &Library) {
    #[cfg(target_os = "macos")]
    {
        type BundleExitFn = unsafe extern "system" fn() -> bool;
        if let Ok(bundle_exit) = unsafe { _lib.get::<BundleExitFn>(b"bundleExit") } {
            unsafe { bundle_exit(); }
        }
    }

    #[cfg(target_os = "linux")]
    {
        type ModuleExitFn = unsafe extern "C" fn() -> bool;
        if let Ok(module_exit) = unsafe { _lib.get::<ModuleExitFn>(b"ModuleExit") } {
            unsafe { module_exit(); }
        }
    }
}
