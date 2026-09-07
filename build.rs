use std::env;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

const GHOSTTY_REVISION: &str = "4c725242b7dbe8c77c6e227ef1f9540c5ef17921";
const GHOSTTY_INCLUDE_DIR_ENV: &str = "GHOSTTY_VT_INCLUDE_DIR";
const GHOSTTY_LIB_DIR_ENV: &str = "GHOSTTY_VT_LIB_DIR";
const CONPTY_VERSION: &str = "1.24.260710001-agent.1";
const CONPTY_FILES: &[&str] = &[
    "conpty.dll",
    "x64/OpenConsole.exe",
    "arm64/OpenConsole.exe",
    "LICENSE.txt",
];

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let upstream = root.join("vendor/ghostty");
    let target = env::var("TARGET").expect("TARGET");

    println!("cargo:rerun-if-env-changed=ZIG");
    println!("cargo:rerun-if-env-changed={GHOSTTY_INCLUDE_DIR_ENV}");
    println!("cargo:rerun-if-env-changed={GHOSTTY_LIB_DIR_ENV}");
    println!("cargo:rerun-if-changed=native/ghostty_bridge.c");
    println!("cargo:rerun-if-changed=native/ghostty_bridge.h");
    println!("cargo:rerun-if-changed=vendor/ghostty/include");
    println!("cargo:rerun-if-changed=vendor/ghostty-patches/terminal-images.patch");
    println!("cargo:rustc-env=AGENT_TERMINAL_CONPTY_VERSION={CONPTY_VERSION}");
    let mut runtime_hash = std::collections::hash_map::DefaultHasher::new();
    for relative in CONPTY_FILES {
        let path = root
            .join("vendor/microsoft-conpty")
            .join(CONPTY_VERSION)
            .join(relative);
        println!("cargo:rerun-if-changed={}", path.display());
        relative.hash(&mut runtime_hash);
        if target.contains("windows") {
            fs::read(&path)
                .expect("read bundled ConPTY runtime input")
                .hash(&mut runtime_hash);
        }
    }
    println!(
        "cargo:rustc-env=AGENT_TERMINAL_CONPTY_HASH_PREFIX={:016x}",
        runtime_hash.finish()
    );

    let ghostty = if env::var_os(GHOSTTY_LIB_DIR_ENV).is_none()
        || env::var_os(GHOSTTY_INCLUDE_DIR_ENV).is_none()
    {
        prepare_ghostty(&root, &upstream)
    } else {
        upstream
    };
    let lib_dir = env::var_os(GHOSTTY_LIB_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            assert!(
                ghostty.join("build.zig").is_file(),
                "missing Ghostty submodule; run git submodule update --init"
            );
            build_ghostty(&ghostty, &target)
        });
    let include_dir = env::var_os(GHOSTTY_INCLUDE_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| ghostty.join("include"));

    link_ghostty(&lib_dir, &target);
    build_bridge(&root, &include_dir);
    println!("cargo:rustc-env=GHOSTTY_SOURCE_REVISION={GHOSTTY_REVISION}");
}

/// Apply the repository-owned extension to a build-owned checkout. The pinned
/// submodule stays untouched, and a clean checkout has everything it needs.
fn prepare_ghostty(root: &Path, upstream: &Path) -> PathBuf {
    assert!(
        upstream.join("build.zig").is_file(),
        "missing Ghostty submodule; run git submodule update --init"
    );
    let patch = root.join("vendor/ghostty-patches/terminal-images.patch");
    let bytes = fs::read(&patch).expect("read terminal image extension patch");
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    GHOSTTY_REVISION.hash(&mut hash);
    bytes.hash(&mut hash);
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let source = out.join(format!("ghostty-source-{:016x}", hash.finish()));
    let ready = source.join(".agent-terminal-patched");
    if ready.is_file() {
        return source;
    }
    // Only an incomplete generated checkout can exist at this content-derived
    // path. Removing it makes interrupted builds safely retryable.
    if source.exists() {
        fs::remove_dir_all(&source).expect("remove incomplete generated Ghostty checkout");
    }
    let status = Command::new("git")
        .args(["clone", "--quiet", "--shared", "--no-checkout"])
        .arg(upstream)
        .arg(&source)
        .status()
        .expect("create build-owned Ghostty checkout");
    assert!(status.success(), "clone pinned Ghostty source: {status}");
    let status = Command::new("git")
        .args(["checkout", "--quiet", "--detach", GHOSTTY_REVISION])
        .current_dir(&source)
        .status()
        .expect("check out pinned Ghostty revision");
    assert!(
        status.success(),
        "check out pinned Ghostty revision: {status}"
    );
    let status = Command::new("git")
        .args(["apply", "--whitespace=nowarn"])
        .arg(&patch)
        .current_dir(&source)
        .status()
        .expect("apply terminal image extension");
    assert!(status.success(), "apply terminal image extension: {status}");
    fs::write(ready, GHOSTTY_REVISION).expect("mark prepared Ghostty source");
    source
}

fn build_ghostty(ghostty: &Path, target: &str) -> PathBuf {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let install = out.join("ghostty-install");
    let cache = out.join("ghostty-zig-cache");
    let zig = env::var_os("ZIG").unwrap_or_else(|| OsString::from("zig"));

    let status = Command::new(zig)
        .arg("build")
        .arg("-Demit-lib-vt=true")
        .arg("-Demit-xcframework=false")
        .arg("-Dapp-runtime=none")
        .arg(format!("-Doptimize={}", optimize_mode()))
        .arg("--prefix")
        .arg(&install)
        .arg("--cache-dir")
        .arg(&cache)
        .arg(format!("-Dtarget={}", zig_target(target)))
        .current_dir(ghostty)
        .status()
        .expect("launch Zig Ghostty build");
    assert!(status.success(), "Ghostty Zig build failed: {status}");

    install.join("lib")
}

fn link_ghostty(lib_dir: &Path, target: &str) {
    if target.contains("windows") {
        let source = lib_dir.join("ghostty-vt-static.lib");
        assert!(source.is_file(), "missing {}", source.display());
        let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
        let link_dir = out.join("ghostty-rust-link");
        fs::create_dir_all(&link_dir).expect("create Windows link directory");
        fs::copy(&source, link_dir.join("ghostty-vt.lib")).expect("copy Ghostty static library");
        println!("cargo:rustc-link-search=native={}", link_dir.display());
        println!("cargo:rustc-link-lib=ntdll");
        println!("cargo:rustc-link-lib=kernel32");
    } else {
        assert!(
            lib_dir.join("libghostty-vt.a").is_file(),
            "missing libghostty-vt.a"
        );
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
    }
    println!("cargo:rustc-link-lib=static=ghostty-vt");
}

fn build_bridge(root: &Path, ghostty_include: &Path) {
    assert!(
        ghostty_include.join("ghostty/vt.h").is_file(),
        "missing Ghostty headers in {}",
        ghostty_include.display()
    );
    cc::Build::new()
        .file(root.join("native/ghostty_bridge.c"))
        .include(root.join("native"))
        .include(ghostty_include)
        .define("GHOSTTY_STATIC", None)
        .warnings(true)
        .compile("ghostty_spike_bridge");
}

fn optimize_mode() -> &'static str {
    if env::var("DEBUG").as_deref() == Ok("true") {
        // Match Cargo's optimized development profile without dropping Zig's
        // runtime safety checks. Unoptimized tests retain the Debug build.
        if env::var("OPT_LEVEL").as_deref() == Ok("0") {
            "Debug"
        } else {
            "ReleaseSafe"
        }
    } else {
        "ReleaseFast"
    }
}

fn zig_target(target: &str) -> &'static str {
    match target {
        "x86_64-unknown-linux-gnu" => "x86_64-linux-gnu",
        "aarch64-unknown-linux-gnu" => "aarch64-linux-gnu",
        "aarch64-apple-darwin" => "aarch64-macos-none",
        "x86_64-apple-darwin" => "x86_64-macos-none",
        "x86_64-pc-windows-msvc" => "x86_64-windows-msvc",
        other => panic!("unsupported target: {other}"),
    }
}
