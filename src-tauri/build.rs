fn main() {
    tauri_build::build();

    // MA0 Spike 修复：cpal 的 Android 后端（oboe）是 C++ 库，其符号
    // （__cxa_pure_virtual 等）需要 libc++ 运行时。NDK 的 lld 默认允许
    // 共享库存在未解析符号，若不显式链接 libc++_shared，dlopen 时报
    // "cannot locate symbol __cxa_pure_virtual" 闪退。
    // 这里用 rustc-link-arg 注入 DT_NEEDED（Tauri CLI 会以
    // CARGO_TARGET_*_RUSTFLAGS 环境变量覆盖 .cargo/config 的 rustflags，
    // build script 的 link-arg 不受影响）；配套的 libc++_shared.so 须
    // 打包进 APK 的 jniLibs/<abi>/（构建脚本见 MA5）。
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg=-lc++_shared");

        // 修复：cpal 链接 aaudio 时，Tauri CLI 默认使用的 NDK 目标 clang（如 aarch64-linux-android24-clang）
        // API level 低于 26，而 aaudio 从 Android API 26 引入，导致链接报错 `unable to find library -laaudio`。
        // 此处将 NDK sysroot 中 API 26 的库路径注入 link-search，确保链接器顺利找到 libaaudio.so。
        if let Ok(ndk) = std::env::var("ANDROID_NDK_HOME")
            .or_else(|_| std::env::var("NDK_HOME"))
            .or_else(|_| std::env::var("ANDROID_HOME").map(|h| format!("{}/ndk/28.2.13676358", h)))
        {
            let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
                Ok("aarch64") => "aarch64-linux-android",
                Ok("arm") => "arm-linux-androideabi",
                Ok("x86") => "i686-linux-android",
                Ok("x86_64") => "x86_64-linux-android",
                _ => "",
            };
            if !arch.is_empty() {
                let prebuilt_dir = std::path::Path::new(&ndk).join("toolchains/llvm/prebuilt");
                if let Ok(entries) = std::fs::read_dir(prebuilt_dir) {
                    for entry in entries.flatten() {
                        let sysroot_lib_26 = entry
                            .path()
                            .join("sysroot")
                            .join("usr")
                            .join("lib")
                            .join(arch)
                            .join("26");
                        if sysroot_lib_26.exists() {
                            println!("cargo:rustc-link-search={}", sysroot_lib_26.display());
                            break;
                        }
                    }
                }
            }
        }
    }
}
