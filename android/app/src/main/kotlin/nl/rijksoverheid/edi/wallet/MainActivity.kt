package nl.rijksoverheid.edi.wallet

import android.os.Bundle
import android.util.Log
import io.flutter.embedding.android.FlutterActivity
import nl.rijksoverheid.edi.wallet.platform_support.hw_keystore.HWKeyStore

class MainActivity : FlutterActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // FIXME: Disabled for now to allow screen sharing during demos
        // Only on Prod. builds to enable screen recording etc. while developing.
        // if (!BuildConfig.DEBUG) window.addFlags(LayoutParams.FLAG_SECURE);
    }
}
