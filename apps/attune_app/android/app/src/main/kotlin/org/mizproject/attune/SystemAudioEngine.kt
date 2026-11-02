package org.mizproject.attune

import android.content.Context
import android.media.AudioAttributes
import android.media.MediaPlayer
import android.os.Handler
import android.os.Looper
import android.util.Log
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.io.FileInputStream

/**
 * SystemAudioEngine manages hardware-accelerated playback via Android's native MediaCodec / MediaPlayer.
 * Acts as the low-overhead fallback engine when the Attune DSP Engine detects system overload,
 * buffer underruns, or device audio clipping.
 */
class SystemAudioEngine private constructor(
    private val context: Context,
    private val channel: MethodChannel
) : MethodChannel.MethodCallHandler {

    companion object {
        private const val CHANNEL_NAME = "org.mizproject.attune/system_audio"
        private const val TAG = "SystemAudioEngine"

        @Volatile
        private var instance: SystemAudioEngine? = null

        fun setup(context: Context, messenger: BinaryMessenger): SystemAudioEngine {
            val chan = MethodChannel(messenger, CHANNEL_NAME)
            val engine = SystemAudioEngine(context, chan)
            chan.setMethodCallHandler(engine)
            instance = engine
            return engine
        }
    }

    private var mediaPlayer: MediaPlayer? = null
    private var currentPath: String? = null
    private val mainHandler = Handler(Looper.getMainLooper())

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "load" -> {
                val path = call.argument<String>("path")
                if (path.isNullOrEmpty()) {
                    result.error("INVALID_PATH", "Path is null or empty", null)
                    return
                }
                val success = loadTrack(path)
                result.success(success)
            }
            "play" -> {
                val success = play()
                result.success(success)
            }
            "pause" -> {
                val success = pause()
                result.success(success)
            }
            "seek" -> {
                val posMs = call.argument<Number>("positionMs")?.toLong() ?: 0L
                val success = seek(posMs)
                result.success(success)
            }
            "stop" -> {
                val success = stop()
                result.success(success)
            }
            "getPosition" -> {
                val pos = getPositionMs()
                result.success(pos)
            }
            "getDuration" -> {
                val dur = getDurationMs()
                result.success(dur)
            }
            "isPlaying" -> {
                val playing = isPlaying()
                result.success(playing)
            }
            "release" -> {
                release()
                result.success(true)
            }
            else -> result.notImplemented()
        }
    }

    private fun loadTrack(path: String): Boolean {
        try {
            release()
            currentPath = path

            val player = MediaPlayer().apply {
                setAudioAttributes(
                    AudioAttributes.Builder()
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .build()
                )

                val file = File(path)
                if (file.exists() && file.canRead()) {
                    val fis = FileInputStream(file)
                    setDataSource(fis.fd)
                    fis.close()
                } else {
                    setDataSource(path)
                }

                setOnCompletionListener {
                    Log.i(TAG, "Track completed in SystemAudioEngine: $currentPath")
                    mainHandler.post {
                        channel.invokeMethod("onTrackEnded", mapOf("path" to currentPath))
                    }
                }

                setOnErrorListener { _, what, extra ->
                    Log.e(TAG, "MediaPlayer error: what=$what, extra=$extra")
                    mainHandler.post {
                        channel.invokeMethod("onError", mapOf("what" to what, "extra" to extra))
                    }
                    true
                }

                prepare()
            }

            mediaPlayer = player
            Log.i(TAG, "Loaded track in SystemAudioEngine: $path (duration: ${player.duration} ms)")
            return true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load track in SystemAudioEngine: $path", e)
            return false
        }
    }

    private fun play(): Boolean {
        return try {
            mediaPlayer?.let {
                if (!it.isPlaying) {
                    it.start()
                    Log.i(TAG, "Playback started in SystemAudioEngine")
                }
                true
            } ?: false
        } catch (e: Exception) {
            Log.e(TAG, "Error starting playback in SystemAudioEngine", e)
            false
        }
    }

    private fun pause(): Boolean {
        return try {
            mediaPlayer?.let {
                if (it.isPlaying) {
                    it.pause()
                    Log.i(TAG, "Playback paused in SystemAudioEngine")
                }
                true
            } ?: false
        } catch (e: Exception) {
            Log.e(TAG, "Error pausing playback in SystemAudioEngine", e)
            false
        }
    }

    private fun seek(positionMs: Long): Boolean {
        return try {
            mediaPlayer?.let {
                it.seekTo(positionMs.toInt())
                true
            } ?: false
        } catch (e: Exception) {
            Log.e(TAG, "Error seeking in SystemAudioEngine", e)
            false
        }
    }

    private fun stop(): Boolean {
        return try {
            mediaPlayer?.let {
                if (it.isPlaying) {
                    it.stop()
                }
                true
            } ?: false
        } catch (e: Exception) {
            Log.e(TAG, "Error stopping playback in SystemAudioEngine", e)
            false
        }
    }

    private fun getPositionMs(): Long {
        return try {
            mediaPlayer?.currentPosition?.toLong() ?: 0L
        } catch (e: Exception) {
            0L
        }
    }

    private fun getDurationMs(): Long {
        return try {
            mediaPlayer?.duration?.toLong() ?: 0L
        } catch (e: Exception) {
            0L
        }
    }

    private fun isPlaying(): Boolean {
        return try {
            mediaPlayer?.isPlaying ?: false
        } catch (e: Exception) {
            false
        }
    }

    fun release() {
        try {
            mediaPlayer?.let {
                if (it.isPlaying) {
                    it.stop()
                }
                it.release()
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error releasing MediaPlayer", e)
        } finally {
            mediaPlayer = null
        }
    }
}
