package com.hao.lumo

import android.content.Context
import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  // LUMO-CUSTOM: cpal/oboe 需要 ndk_context（ndk-glue 风格）初始化的 Android 上下文，
  // Tauri 2 的胶水层（wry）不提供。这里在 Activity 创建时把 Context 经 JNI 传给
  // Rust 侧的 initialize_android_context（见 src-tauri/src/lib.rs）。没有它，
  // 任何音频初始化都会 panic: "android context was not initialized"。
  private external fun initLumoAudioContext(context: Context)

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    runCatching { initLumoAudioContext(applicationContext) }
  }
}
