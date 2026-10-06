package app.restropro.btprinter

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothClass
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothSocket
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.util.Base64
import androidx.core.content.ContextCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.IOException
import java.util.UUID
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

@InvokeArg
class AddressArgs {
    lateinit var address: String
}

@InvokeArg
class PrintArgs {
    lateinit var address: String
    lateinit var data: String // base64 ESC/POS bytes
}

/**
 * Raw printing to a classic-Bluetooth (SPP / RFCOMM) receipt printer that is already paired in
 * Android settings. Mirrors the Windows COM-port path: open, write ESC/POS bytes in small
 * chunks, let the printer drain, close.
 *
 * Called only from Rust (see mobile.rs), never from JavaScript.
 */
@TauriPlugin(
    permissions = [
        Permission(strings = [Manifest.permission.BLUETOOTH_CONNECT], alias = "bluetooth")
    ]
)
class BtPrinterPlugin(private val activity: Activity) : Plugin(activity) {

    // One job at a time: two receipts must never interleave on one printer, and a Bluetooth
    // connect() blocks for seconds, so none of this may run on the main thread.
    private val io: ExecutorService = Executors.newSingleThreadExecutor()

    companion object {
        // Well-known Serial Port Profile UUID.
        private val SPP_UUID: UUID = UUID.fromString("00001101-0000-1000-8000-00805F9B34FB")

        // Same tuning as the Windows shell (serial_print.rs): mini printers have a tiny receive
        // buffer and drop data if it arrives too fast.
        private const val CHUNK = 256
        private const val CHUNK_PAUSE_MS = 25L

        // Closing an RFCOMM socket straight after write() can discard what the printer has not
        // yet received, so the last lines of a receipt go missing. Windows waits 400 ms; Android's
        // close is blunter, so wait a little longer.
        private const val DRAIN_MS = 600L

        private val PRINTER_NAME = Regex(
            "print|pos|rpp|mpt|mtp|ptp|pt-|zj-|xp-|qs-|hm-|bt-?p|thermal|receipt",
            RegexOption.IGNORE_CASE
        )
    }

    private fun adapter(): BluetoothAdapter? =
        (activity.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter

    private fun hasPermission(): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.S ||
            ContextCompat.checkSelfPermission(activity, Manifest.permission.BLUETOOTH_CONNECT) ==
            PackageManager.PERMISSION_GRANTED

    private fun granted(value: Boolean): JSObject {
        val o = JSObject()
        o.put("granted", value)
        return o
    }

    @Command
    fun ensurePermission(invoke: Invoke) {
        if (hasPermission()) {
            invoke.resolve(granted(true))
        } else {
            requestPermissionForAlias("bluetooth", invoke, "onPermissionResult")
        }
    }

    @PermissionCallback
    fun onPermissionResult(invoke: Invoke) {
        invoke.resolve(granted(hasPermission()))
    }

    @SuppressLint("MissingPermission") // ensurePermission() runs first on the Rust side
    @Command
    fun listPaired(invoke: Invoke) {
        val adapter = adapter()
        if (adapter == null) {
            invoke.reject("This device has no Bluetooth. Use a network (Wi-Fi/LAN) printer instead.")
            return
        }
        if (!adapter.isEnabled) {
            invoke.reject("Bluetooth is turned off. Turn it on, and pair the printer in Android settings (Bluetooth).")
            return
        }
        try {
            val devices = adapter.bondedDevices.orEmpty()
                .map { Triple(it, it.name ?: it.address, likelyPrinter(it)) }
                .sortedWith(compareByDescending<Triple<BluetoothDevice, String, Boolean>> { it.third }.thenBy { it.second.lowercase() })
            val arr = JSArray()
            for ((device, name, likely) in devices) {
                val o = JSObject()
                o.put("address", device.address)
                o.put("name", name)
                o.put("likelyPrinter", likely)
                arr.put(o)
            }
            val out = JSObject()
            out.put("devices", arr)
            invoke.resolve(out)
        } catch (e: SecurityException) {
            invoke.reject("Bluetooth permission was denied. Allow \"Nearby devices\" for Restro Pro POS in Android settings.")
        }
    }

    @SuppressLint("MissingPermission") // ensurePermission() runs first on the Rust side
    @Command
    fun status(invoke: Invoke) {
        val args = invoke.parseArgs(AddressArgs::class.java)
        val out = JSObject()
        val adapter = adapter()
        when {
            adapter == null -> {
                out.put("present", false)
                out.put("detail", "This device has no Bluetooth.")
            }
            !adapter.isEnabled -> {
                out.put("present", false)
                out.put("detail", "Bluetooth is turned off.")
            }
            !BluetoothAdapter.checkBluetoothAddress(args.address) -> {
                out.put("present", false)
                out.put("detail", "\"${args.address}\" is not a Bluetooth address. Pick the printer again in printer settings.")
            }
            else -> try {
                val bonded = adapter.getRemoteDevice(args.address).bondState == BluetoothDevice.BOND_BONDED
                out.put("present", bonded)
                out.put(
                    "detail",
                    if (bonded) "Printer is paired. Switch it on and keep it close before printing."
                    else "Printer is not paired. Pair it in Android settings (Bluetooth) first."
                )
            } catch (e: SecurityException) {
                out.put("present", false)
                out.put("detail", "Bluetooth permission was denied.")
            }
        }
        invoke.resolve(out)
    }

    @Command
    fun print(invoke: Invoke) {
        val args = invoke.parseArgs(PrintArgs::class.java)
        io.execute {
            try {
                printNow(args.address, Base64.decode(args.data, Base64.DEFAULT))
                invoke.resolve()
            } catch (e: PrintFailure) {
                invoke.reject(e.message ?: "Printing failed.")
            } catch (e: SecurityException) {
                invoke.reject("Bluetooth permission was denied. Allow \"Nearby devices\" for Restro Pro POS in Android settings.")
            } catch (e: Exception) {
                invoke.reject("Printing failed — ${e.message ?: e.javaClass.simpleName}")
            }
        }
    }

    private class PrintFailure(message: String) : Exception(message)

    @SuppressLint("MissingPermission")
    private fun printNow(address: String, data: ByteArray) {
        val adapter = adapter() ?: throw PrintFailure("This device has no Bluetooth.")
        if (!adapter.isEnabled) throw PrintFailure("Bluetooth is turned off.")
        if (!BluetoothAdapter.checkBluetoothAddress(address)) {
            throw PrintFailure("\"$address\" is not a Bluetooth address. Pick the printer again in printer settings.")
        }
        val device = adapter.getRemoteDevice(address)
        if (device.bondState != BluetoothDevice.BOND_BONDED) {
            throw PrintFailure("The printer is not paired. Pair it in Android settings (Bluetooth) first.")
        }

        // A running discovery slows connect() a lot. Cancelling needs BLUETOOTH_SCAN on Android 12+,
        // which we deliberately do not hold, so a refusal is fine: just carry on.
        try {
            adapter.cancelDiscovery()
        } catch (_: SecurityException) {
        }

        val socket = connect(device)
        try {
            val out = socket.outputStream
            var offset = 0
            while (offset < data.size) {
                val end = minOf(offset + CHUNK, data.size)
                out.write(data, offset, end - offset)
                out.flush()
                offset = end
                if (offset < data.size) Thread.sleep(CHUNK_PAUSE_MS)
            }
            Thread.sleep(DRAIN_MS)
        } catch (e: IOException) {
            throw PrintFailure("Connected to the printer but the print job failed partway — ${e.message}")
        } finally {
            try {
                socket.close()
            } catch (_: IOException) {
            }
        }
    }

    @SuppressLint("MissingPermission")
    private fun connect(device: BluetoothDevice): BluetoothSocket {
        var last: IOException? = null
        // Secure first; some printers only accept the insecure variant.
        for (insecure in listOf(false, true)) {
            val socket = try {
                if (insecure) device.createInsecureRfcommSocketToServiceRecord(SPP_UUID)
                else device.createRfcommSocketToServiceRecord(SPP_UUID)
            } catch (e: IOException) {
                last = e
                continue
            }
            try {
                socket.connect()
                return socket
            } catch (e: IOException) {
                last = e
                try {
                    socket.close()
                } catch (_: IOException) {
                }
            }
        }
        throw PrintFailure(
            "Could not connect to the printer — ${last?.message ?: "unknown error"}. " +
                "Check that it is switched on, in range, and not connected to another phone."
        )
    }

    @SuppressLint("MissingPermission")
    private fun likelyPrinter(d: BluetoothDevice): Boolean {
        val major = d.bluetoothClass?.majorDeviceClass
        return major == BluetoothClass.Device.Major.IMAGING || PRINTER_NAME.containsMatchIn(d.name ?: "")
    }
}
