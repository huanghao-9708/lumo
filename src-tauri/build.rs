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
    }
}
