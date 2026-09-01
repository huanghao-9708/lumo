package com.hao.lumo

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat

/**
 * LUMO-CUSTOM (MA2): Android 前台服务与系统媒体通知集成。
 *
 * 职责：
 * 1. 锁屏/灭屏下持有 MediaSession 与常驻媒体通知，避免进程被杀
 * 2. 处理通知五键控制与系统媒体按键（蓝牙/锁屏），经 JNI 回传 Rust 队列
 * 3. 监听音频焦点（来电/其他应用抢占）与耳机拔出（BECOMING_NOISY）
 */
class MediaPlaybackService : Service() {

    private var mediaSession: MediaSession? = null
    private var audioManager: AudioManager? = null
    private var focusRequest: AudioFocusRequest? = null
    private var noisyReceiverRegistered = false

    private val noisyReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == AudioManager.ACTION_AUDIO_BECOMING_NOISY) {
                // 拔出耳机：通知 Rust 暂停播放
                lumoNativeMediaAction(ACTION_PAUSE, 0)
            }
        }
    }

    private val focusChangeListener = AudioManager.OnAudioFocusChangeListener { focusChange ->
        when (focusChange) {
            AudioManager.AUDIOFOCUS_LOSS,
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> {
                // 失去焦点（来电等）：暂停播放
                lumoNativeMediaAction(ACTION_PAUSE, 0)
            }
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> {
                // 导航短暂语音等：可适当静音/暂停
                lumoNativeMediaAction(ACTION_PAUSE, 0)
            }
            AudioManager.AUDIOFOCUS_GAIN -> {
                // 重新获得焦点：依 MA2 设计不自动恢复，保持暂停免打扰
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        setupMediaSession()
        requestAudioFocus()
        registerNoisyReceiver()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Lumo 音乐播放",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "正在播放的音乐控制通知"
                setShowBadge(false)
                setSound(null, null)
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager?.createNotificationChannel(channel)
        }
    }

    private fun setupMediaSession() {
        mediaSession = MediaSession(this, "LumoMediaSession").apply {
            setCallback(object : MediaSession.Callback() {
                override fun onPlay() {
                    lumoNativeMediaAction(ACTION_PLAY, 0)
                }

                override fun onPause() {
                    lumoNativeMediaAction(ACTION_PAUSE, 0)
                }

                override fun onSkipToNext() {
                    lumoNativeMediaAction(ACTION_NEXT, 0)
                }

                override fun onSkipToPrevious() {
                    lumoNativeMediaAction(ACTION_PREV, 0)
                }

                override fun onSeekTo(pos: Long) {
                    lumoNativeMediaAction(ACTION_SEEK, pos)
                }

                override fun onStop() {
                    lumoNativeMediaAction(ACTION_STOP, 0)
                }
            })
            isActive = true
        }
    }

    private fun requestAudioFocus() {
        audioManager = getSystemService(Context.AUDIO_SERVICE) as? AudioManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val attrs = AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_MEDIA)
                .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                .build()
            focusRequest = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(attrs)
                .setOnAudioFocusChangeListener(focusChangeListener)
                .build()
            focusRequest?.let { audioManager?.requestAudioFocus(it) }
        } else {
            @Suppress("DEPRECATION")
            audioManager?.requestAudioFocus(
                focusChangeListener,
                AudioManager.STREAM_MUSIC,
                AudioManager.AUDIOFOCUS_GAIN
            )
        }
    }

    private fun abandonAudioFocus() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            focusRequest?.let { audioManager?.abandonAudioFocusRequest(it) }
        } else {
            @Suppress("DEPRECATION")
            audioManager?.abandonAudioFocus(focusChangeListener)
        }
    }

    private fun registerNoisyReceiver() {
        if (!noisyReceiverRegistered) {
            val filter = IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY)
            registerReceiver(noisyReceiver, filter)
            noisyReceiverRegistered = true
        }
    }

    private fun unregisterNoisyReceiver() {
        if (noisyReceiverRegistered) {
            unregisterReceiver(noisyReceiver)
            noisyReceiverRegistered = false
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent == null) return START_NOT_STICKY

        val action = intent.action
        if (action == ACTION_STOP_SERVICE) {
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
            return START_NOT_STICKY
        }

        val title = intent.getStringExtra(EXTRA_TITLE) ?: "Lumo Music"
        val artist = intent.getStringExtra(EXTRA_ARTIST) ?: "未知艺人"
        val album = intent.getStringExtra(EXTRA_ALBUM) ?: ""
        val isPlaying = intent.getBooleanExtra(EXTRA_IS_PLAYING, false)
        val positionMs = intent.getLongExtra(EXTRA_POSITION_MS, 0L)
        val durationMs = intent.getLongExtra(EXTRA_DURATION_MS, 0L)

        updateMediaSessionState(isPlaying, positionMs)
        val notification = buildNotification(title, artist, album, isPlaying)
        startForeground(NOTIFICATION_ID, notification)

        return START_STICKY
    }

    private fun updateMediaSessionState(isPlaying: Boolean, positionMs: Long) {
        val stateBuilder = PlaybackState.Builder()
            .setActions(
                PlaybackState.ACTION_PLAY or
                PlaybackState.ACTION_PAUSE or
                PlaybackState.ACTION_SKIP_TO_NEXT or
                PlaybackState.ACTION_SKIP_TO_PREVIOUS or
                PlaybackState.ACTION_SEEK_TO or
                PlaybackState.ACTION_STOP
            )
            .setState(
                if (isPlaying) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED,
                positionMs,
                if (isPlaying) 1.0f else 0.0f
            )
        mediaSession?.setPlaybackState(stateBuilder.build())
    }

    private fun buildNotification(
        title: String,
        artist: String,
        album: String,
        isPlaying: Boolean
    ): Notification {
        val clickIntent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        val contentPendingIntent = PendingIntent.getActivity(
            this,
            0,
            clickIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val prevPending = createActionPendingIntent(ACTION_PREV)
        val playPausePending = createActionPendingIntent(if (isPlaying) ACTION_PAUSE else ACTION_PLAY)
        val nextPending = createActionPendingIntent(ACTION_NEXT)

        val builder = NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(title)
            .setContentText("$artist · $album")
            .setSmallIcon(android.R.drawable.ic_media_play)
            .setContentIntent(contentPendingIntent)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setOngoing(isPlaying)
            .addAction(android.R.drawable.ic_media_previous, "上一首", prevPending)
            .addAction(
                if (isPlaying) android.R.drawable.ic_media_pause else android.R.drawable.ic_media_play,
                if (isPlaying) "暂停" else "播放",
                playPausePending
            )
            .addAction(android.R.drawable.ic_media_next, "下一首", nextPending)

        // 使用系统 MediaStyle 样式
        val style = androidx.media.app.NotificationCompat.MediaStyle()
            .setShowActionsInCompactView(0, 1, 2)
        mediaSession?.let {
            style.setMediaSession(android.support.v4.media.session.MediaSessionCompat.Token.fromToken(it.sessionToken))
        }
        builder.setStyle(style)

        return builder.build()
    }

    private fun createActionPendingIntent(actionCode: Int): PendingIntent {
        val intent = Intent(this, MediaActionReceiver::class.java).apply {
            action = ACTION_MEDIA_CONTROL
            putExtra(EXTRA_ACTION_CODE, actionCode)
        }
        return PendingIntent.getBroadcast(
            this,
            actionCode,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
    }

    override fun onDestroy() {
        abandonAudioFocus()
        unregisterNoisyReceiver()
        mediaSession?.apply {
            isActive = false
            release()
        }
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    companion object {
        const val CHANNEL_ID = "lumo_playback_channel"
        const val NOTIFICATION_ID = 4202

        const val ACTION_PLAY = 1
        const val ACTION_PAUSE = 2
        const val ACTION_NEXT = 3
        const val ACTION_PREV = 4
        const val ACTION_SEEK = 5
        const val ACTION_STOP = 6

        const val ACTION_MEDIA_CONTROL = "com.hao.lumo.ACTION_MEDIA_CONTROL"
        const val ACTION_STOP_SERVICE = "com.hao.lumo.ACTION_STOP_SERVICE"

        const val EXTRA_TITLE = "title"
        const val EXTRA_ARTIST = "artist"
        const val EXTRA_ALBUM = "album"
        const val EXTRA_IS_PLAYING = "is_playing"
        const val EXTRA_POSITION_MS = "position_ms"
        const val EXTRA_DURATION_MS = "duration_ms"
        const val EXTRA_ACTION_CODE = "action_code"

        @JvmStatic
        fun update(
            context: Context,
            title: String,
            artist: String,
            album: String,
            isPlaying: Boolean,
            positionMs: Long,
            durationMs: Long
        ) {
            val intent = Intent(context, MediaPlaybackService::class.java).apply {
                putExtra(EXTRA_TITLE, title)
                putExtra(EXTRA_ARTIST, artist)
                putExtra(EXTRA_ALBUM, album)
                putExtra(EXTRA_IS_PLAYING, isPlaying)
                putExtra(EXTRA_POSITION_MS, positionMs)
                putExtra(EXTRA_DURATION_MS, durationMs)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        @JvmStatic
        fun stop(context: Context) {
            val intent = Intent(context, MediaPlaybackService::class.java).apply {
                action = ACTION_STOP_SERVICE
            }
            context.startService(intent)
        }
    }
}

/**
 * 接收来自通知栏 PendingIntent 的媒体动作广播，并直接调用 Rust 导出函数
 */
class MediaActionReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context?, intent: Intent?) {
        if (intent?.action == MediaPlaybackService.ACTION_MEDIA_CONTROL) {
            val actionCode = intent.getIntExtra(MediaPlaybackService.EXTRA_ACTION_CODE, 0)
            if (actionCode > 0) {
                lumoNativeMediaAction(actionCode, 0)
            }
        }
    }
}

// JNI 导出声明：由 Rust 导出函数接收
private external fun lumoNativeMediaAction(action: Int, param: Long)
