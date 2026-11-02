package org.mizproject.attune

import android.content.Context
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.EventChannel

/**
 * Streams accelerometer tilt events to Flutter for liquid glass 3D parallax effects.
 * Automatically halts hardware sensor polling when unsubscribed/canceled (e.g. when
 * screen is not in view) to guarantee zero CPU/battery drain.
 */
class SensorStreamHandler(context: Context) : EventChannel.StreamHandler, SensorEventListener {
    private val sensorManager = context.getSystemService(Context.SENSOR_SERVICE) as? SensorManager
    private val accelerometer = sensorManager?.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)
    private var eventSink: EventChannel.EventSink? = null

    // Low-pass filter smoothing values
    private var smoothX = 0.0f
    private var smoothY = 0.0f
    private val alpha = 0.15f // Smoothing factor

    companion object {
        private const val CHANNEL_NAME = "org.mizproject.attune/sensors/accelerometer"

        fun setup(context: Context, messenger: BinaryMessenger) {
            val channel = EventChannel(messenger, CHANNEL_NAME)
            channel.setStreamHandler(SensorStreamHandler(context))
        }
    }

    override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
        this.eventSink = events
        accelerometer?.let {
            sensorManager?.registerListener(this, it, SensorManager.SENSOR_DELAY_UI)
        }
    }

    override fun onCancel(arguments: Any?) {
        sensorManager?.unregisterListener(this)
        this.eventSink = null
    }

    override fun onSensorChanged(event: SensorEvent?) {
        if (event == null || event.sensor.type != Sensor.TYPE_ACCELEROMETER) return

        val rawX = event.values[0]
        val rawY = event.values[1]

        // Low-pass filter to smooth hand tremors
        smoothX = smoothX + alpha * (rawX - smoothX)
        smoothY = smoothY + alpha * (rawY - smoothY)

        // Normalize to approx [-1.0, 1.0] range (gravity ~9.81 m/s^2)
        // Negate X so tilting right tilts the glass right
        val normX = (-smoothX / 9.81f).coerceIn(-1.0f, 1.0f)
        val normY = (smoothY / 9.81f).coerceIn(-1.0f, 1.0f)

        eventSink?.success(doubleArrayOf(normX.toDouble(), normY.toDouble()))
    }

    override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) {
        // No-op
    }
}
