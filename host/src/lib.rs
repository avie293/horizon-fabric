//! Runs Fabric Loader in a JVM inside the game's process: Rust starts the JVM, Fabric Loader finds the
//! mods through `HorizonGameProvider`, and the mods see a facade of Minecraft's classes whose methods
//! come back here. See the repository's `README.md`.

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::sys::{self, jint};
use jni::{JNIEnv, JavaVM, NativeMethod};
use std::ffi::{CString, c_void};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const BOOTSTRAP: &str = "dev/horizonlab/fabric/loader/Bootstrap";

/// What the host needs to start. Everything is a path on the player's machine.
#[derive(Clone, Debug)]
pub struct Config {
    /// A Java 25 or later: the Java 26.3 is built for.
    pub java_home: PathBuf,
    /// Fabric Loader, its libraries and the provider's jar.
    pub classpath: Vec<PathBuf>,
    /// The facade of Minecraft's classes. Not on the class path: Knot loads it, so Mixin can patch it.
    pub game_jar: PathBuf,
    /// The libraries the real game ships, which mods use as their own (slf4j, Guava, LWJGL...).
    pub game_libraries: Vec<PathBuf>,
    /// Holds `mods/` and `config/`.
    pub game_dir: PathBuf,
}

/// Why the host couldn't do something. A Java exception comes with its stack trace.
#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// What the mods asked of the game during a call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outgoing {
    SystemMessage(String),
}

/// Filled by native methods on whichever thread calls into Java, and emptied by that call.
static OUTGOING: Mutex<Vec<Outgoing>> = Mutex::new(Vec::new());

fn push_outgoing(message: Outgoing) {
    OUTGOING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(message);
}

fn take_outgoing() -> Vec<Outgoing> {
    std::mem::take(&mut *OUTGOING.lock().unwrap_or_else(|e| e.into_inner()))
}

/// The JVM with Fabric Loader booted and the mods initialised. A process has one JVM for good.
pub struct FabricHost {
    vm: JavaVM,
    game: GlobalRef,
}

impl FabricHost {
    pub fn start(config: &Config) -> Result<FabricHost, Error> {
        let vm = create_vm(config)?;
        let mut env = vm
            .attach_current_thread_permanently()
            .map_err(|e| Error(format!("attaching to the JVM: {e}")))?;

        let game_dir = env
            .new_string(config.game_dir.to_string_lossy())
            .map_err(|e| java_error(&mut env, e))?;
        let game = env
            .call_static_method(
                BOOTSTRAP,
                "boot",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&game_dir)],
            )
            .and_then(|v| v.l())
            .map_err(|e| java_error(&mut env, e))?;
        let game = env
            .new_global_ref(game)
            .map_err(|e| java_error(&mut env, e))?;

        let class: &JClass = game.as_obj().into();
        env.register_native_methods(
            class,
            &[NativeMethod {
                name: "nativeSystemMessage".into(),
                sig: "(Ljava/lang/String;)V".into(),
                fn_ptr: native_system_message as *mut c_void,
            }],
        )
        .map_err(|e| java_error(&mut env, e))?;
        env.call_static_method(class, "start", "()V", &[])
            .map_err(|e| java_error(&mut env, e))?;

        Ok(FabricHost { vm, game })
    }

    /// One client tick for the mods, on the thread that calls it. Returns what they asked for.
    pub fn client_tick(&self, in_world: bool) -> Result<Vec<Outgoing>, Error> {
        let mut env = self
            .vm
            .attach_current_thread_permanently()
            .map_err(|e| Error(format!("attaching to the JVM: {e}")))?;
        let class: &JClass = self.game.as_obj().into();
        env.call_static_method(
            class,
            "clientTick",
            "(Z)V",
            &[JValue::Bool(in_world.into())],
        )
        .map_err(|e| java_error(&mut env, e))?;
        Ok(take_outgoing())
    }
}

extern "system" fn native_system_message(mut env: JNIEnv, _class: JClass, text: JString) {
    if let Ok(text) = env.get_string(&text) {
        push_outgoing(Outgoing::SystemMessage(text.into()));
    }
}

/// Where a JDK or JRE keeps the JVM's library.
fn jvm_library(java_home: &Path) -> PathBuf {
    if cfg!(windows) {
        java_home.join("bin/server/jvm.dll")
    } else if cfg!(target_os = "macos") {
        java_home.join("lib/server/libjvm.dylib")
    } else {
        java_home.join("lib/server/libjvm.so")
    }
}

fn load_jvm(path: &Path) -> Result<libloading::Library, libloading::Error> {
    // The JVM's library finds its neighbours (the C runtime in bin/) only with this flag.
    #[cfg(windows)]
    unsafe {
        use libloading::os::windows::{LOAD_WITH_ALTERED_SEARCH_PATH, Library};
        Library::load_with_flags(path, LOAD_WITH_ALTERED_SEARCH_PATH).map(Into::into)
    }
    #[cfg(not(windows))]
    unsafe {
        libloading::Library::new(path)
    }
}

fn jvm_options(config: &Config) -> Result<Vec<CString>, Error> {
    let separator = if cfg!(windows) { ";" } else { ":" };
    let joined = |paths: &[PathBuf]| {
        paths
            .iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join(separator)
    };
    [
        format!("-Djava.class.path={}", joined(&config.classpath)),
        format!("-Dhorizon.fabric.gameJar={}", config.game_jar.display()),
        format!(
            "-Dhorizon.fabric.gameLibraries={}",
            joined(&config.game_libraries)
        ),
        // The Minecraft provider in Fabric Loader's jar stands aside for ours.
        "-Dfabric.skipMcProvider=true".to_string(),
        // The JVM keeps its hands off the game's signal and console handlers.
        "-Xrs".to_string(),
    ]
    .into_iter()
    .map(|option| CString::new(option).map_err(|e| Error(format!("JVM option: {e}"))))
    .collect()
}

fn create_vm(config: &Config) -> Result<JavaVM, Error> {
    let path = jvm_library(&config.java_home);
    let library = load_jvm(&path).map_err(|e| Error(format!("loading {}: {e}", path.display())))?;

    type CreateVm =
        unsafe extern "system" fn(*mut *mut sys::JavaVM, *mut *mut c_void, *mut c_void) -> jint;
    let create: libloading::Symbol<CreateVm> = unsafe { library.get(b"JNI_CreateJavaVM\0") }
        .map_err(|e| Error(format!("{}: {e}", path.display())))?;

    let options = jvm_options(config)?;
    let mut raw_options: Vec<sys::JavaVMOption> = options
        .iter()
        .map(|option| sys::JavaVMOption {
            optionString: option.as_ptr() as *mut _,
            extraInfo: std::ptr::null_mut(),
        })
        .collect();
    let mut args = sys::JavaVMInitArgs {
        version: sys::JNI_VERSION_1_8,
        nOptions: raw_options.len() as jint,
        options: raw_options.as_mut_ptr(),
        ignoreUnrecognized: sys::JNI_FALSE,
    };

    let mut vm: *mut sys::JavaVM = std::ptr::null_mut();
    let mut env: *mut c_void = std::ptr::null_mut();
    let code = unsafe { create(&mut vm, &mut env, &mut args as *mut _ as *mut c_void) };
    if code != sys::JNI_OK {
        return Err(Error(format!("JNI_CreateJavaVM returned {code}")));
    }
    // The JVM can't be unloaded, and its library stays for as long as the process does.
    std::mem::forget(library);
    unsafe { JavaVM::from_raw(vm) }.map_err(|e| Error(format!("the JVM: {e}")))
}

/// Turns a failed JNI call into an error, with the stack trace of the Java exception behind it.
fn java_error(env: &mut JNIEnv, error: jni::errors::Error) -> Error {
    if !env.exception_check().unwrap_or(false) {
        return Error(error.to_string());
    }
    let Ok(thrown) = env.exception_occurred() else {
        return Error(error.to_string());
    };
    let _ = env.exception_clear();
    let thrown: JObject = thrown.into();
    let described = env
        .call_static_method(
            BOOTSTRAP,
            "describe",
            "(Ljava/lang/Throwable;)Ljava/lang/String;",
            &[JValue::Object(&thrown)],
        )
        .and_then(|v| v.l());
    match described {
        Ok(text) => {
            let text = JString::from(text);
            match env.get_string(&text) {
                Ok(text) => Error(text.into()),
                Err(e) => Error(e.to_string()),
            }
        }
        Err(e) => {
            let _ = env.exception_clear();
            Error(format!("{error}; describing it: {e}"))
        }
    }
}
