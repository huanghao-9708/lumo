package com.hao.lumo

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

// LUMO-CUSTOM: 以下均为 Lumo 平台桥（MA1）。
// - initLumoAudioContext: 把 Activity 传给 Rust（ndk_context + 平台桥共用一份 GlobalRef）
// - lumoNativeOnBackPressed / lumoNativePermissionResult: Kotlin → Rust 事件回注
// - lumoHasAudioPermission / lumoRequestAudioPermission / lumoOpenAppSettings /
//   lumoFinishApp: Rust 经 JNI 调用的平台辅助（见 src-tauri/src/services/platform.rs）
class MainActivity : TauriActivity() {
  private external fun initLumoAudioContext(context: android.content.Context)
  private external fun lumoNativeOnBackPressed()
  private external fun lumoNativePermissionResult(granted: Boolean)

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    runCatching { initLumoAudioContext(this) }

    // 返回键不直接退出：交给 WebView 视图栈处理（栈空时由 Rust 调 lumoFinishApp）
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        runCatching { lumoNativeOnBackPressed() }
      }
    })
  }

  fun lumoHasAudioPermission(): Boolean {
    val perm = if (Build.VERSION.SDK_INT >= 33) Manifest.permission.READ_MEDIA_AUDIO
               else Manifest.permission.READ_EXTERNAL_STORAGE
    return ContextCompat.checkSelfPermission(this, perm) == PackageManager.PERMISSION_GRANTED
  }

  fun lumoRequestAudioPermission() {
    val perm = if (Build.VERSION.SDK_INT >= 33) Manifest.permission.READ_MEDIA_AUDIO
               else Manifest.permission.READ_EXTERNAL_STORAGE
    ActivityCompat.requestPermissions(this, arrayOf(perm), LUMO_PERM_REQ)
  }

  override fun onRequestPermissionsResult(
    requestCode: Int,
    permissions: Array<out String>,
    grantResults: IntArray
  ) {
    super.onRequestPermissionsResult(requestCode, permissions, grantResults)
    if (requestCode == LUMO_PERM_REQ) {
      val granted = grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED
      runCatching { lumoNativePermissionResult(granted) }
    }
  }

  fun lumoOpenAppSettings() {
    startActivity(
      Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", packageName, null))
    )
  }

  fun lumoFinishApp() {
    finishAffinity()
  }

  private companion object {
    const val LUMO_PERM_REQ = 4201
  }
}
