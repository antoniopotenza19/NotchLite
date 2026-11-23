// Original NotchLite companion code. SPDX-License-Identifier: Apache-2.0
package it.notchlite.companion

import android.app.AlertDialog
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import android.view.View
import android.widget.*
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.lifecycle.lifecycleScope
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import kotlinx.coroutines.*
import org.json.JSONObject

class MainActivity : ComponentActivity() {
    private lateinit var qrField: EditText
    private lateinit var message: TextView
    private lateinit var pairButton: Button
    private lateinit var sendButton: Button
    private lateinit var receiveButton: Button
    private lateinit var ackButton: Button
    private var api: ApiClient? = null
    private var token: String? = null
    private var pairId: String? = null
    private var pollToken: String? = null
    private var outgoing: JSONObject? = null
    private var transferId: String? = null
    private var ackId: String? = null
    private var job: Job? = null
    private var foreground = false
    private var busy = false
    private var generation = 0
    private var pendingAction: (suspend () -> Unit)? = null
    private var awaitingExport: JSONObject? = null
    private val scan = registerForActivityResult(ScanContract()) { result -> result.contents?.let { qrField.setText(it) } }
    private val pick = registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> if (uri != null) send(uri) }
    private val save = registerForActivityResult(ActivityResultContracts.CreateDocument("application/octet-stream")) { uri ->
        val offer = awaitingExport; awaitingExport = null
        if (uri != null && offer != null) receive(uri, offer)
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val padding = (24 * resources.displayMetrics.density).toInt()
        val column = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(padding, padding, padding, padding); setBackgroundColor(Color.rgb(24, 29, 38)) }
        fun label(text: String, size: Float = 16f): TextView = TextView(this).apply { this.text = text; textSize = size; setTextColor(Color.rgb(244, 247, 252)); setPadding(0, 12, 0, 12); column.addView(this) }
        fun button(text: String, action: () -> Unit): Button = Button(this).apply { this.text = text; setOnClickListener { action() }; column.addView(this) }
        label("NotchLite Companion", 26f)
        label("Graphite · rete locale\nApri Telefono sul PC, scansiona il QR e confronta il codice. Limite file: 256 MiB.")
        qrField = EditText(this).apply { hint = "Incolla notchlite://pair?..."; setTextColor(Color.WHITE); setHintTextColor(Color.LTGRAY); isSaveEnabled = false; maxLines = 4; column.addView(this) }
        button("Scansiona QR") { scan.launch(ScanOptions().setDesiredBarcodeFormats(ScanOptions.QR_CODE).setBeepEnabled(false).setPrompt("Inquadra il QR del PC")) }
        pairButton = button("Associa al PC") { pair() }
        message = label("Nessun PC associato")
        sendButton = button("Invia un file al PC") { pick.launch(arrayOf("*/*")) }
        receiveButton = button("Ricevi e salva file dal PC") {
            outgoing?.let { awaitingExport = it; save.launch(safeName(it.getString("name"))) }
        }
        ackButton = button("Conferma file già salvato") { acknowledge() }
        button("Annulla trasferimento") { cancelTransfer() }
        button("Disconnetti") { disconnect(); message.text = "Disconnesso. Per riprovare genera un nuovo QR sul PC." }
        button("Licenze open source") {
            val notices = assets.open("NOTICE.txt").bufferedReader().use { it.readText() }
            AlertDialog.Builder(this).setTitle("Licenze").setMessage(notices).setPositiveButton("Chiudi", null).show()
        }
        setContentView(ScrollView(this).apply {
            setBackgroundColor(Color.rgb(24, 29, 38)); addView(column)
            setOnApplyWindowInsetsListener { view, insets ->
                @Suppress("DEPRECATION")
                view.setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop, insets.systemWindowInsetRight, insets.systemWindowInsetBottom)
                insets
            }
        }); refresh()
    }
    override fun onStart() {
        super.onStart(); foreground = true
        val pending = pendingAction; pendingAction = null
        if (pending != null) work(pending) else poll()
    }
    override fun onStop() {
        foreground = false; generation++; job?.cancel(); api?.abort(); job = null; busy = false; refresh()
        super.onStop()
    }
    override fun onDestroy() { disconnect(); super.onDestroy() }
    private fun refresh() {
        val paired = token != null
        pairButton.isEnabled = !busy && !paired && pairId == null
        sendButton.isEnabled = paired && !busy && transferId == null
        receiveButton.isEnabled = paired && !busy && outgoing != null && ackId == null
        ackButton.visibility = if (ackId != null) View.VISIBLE else View.GONE; ackButton.isEnabled = !busy
    }
    private fun work(action: suspend () -> Unit) {
        if (!foreground) { pendingAction = action; return }
        val run = ++generation
        job?.cancel(); api?.abort(); busy = true; refresh()
        job = lifecycleScope.launch {
            try { currentCoroutineContext().ensureActive(); if (run != generation) throw CancellationException(); action() }
            catch (e: CancellationException) { throw e }
            catch (e: Exception) { if (run == generation) message.text = "Operazione non completata: ${e.message ?: "errore di connessione"}" }
            finally { if (run == generation) { busy = false; job = null; refresh(); poll() } }
        }
    }
    private suspend fun request(method: String, path: String, bearer: String? = token, body: JSONObject? = null): JSONObject {
        currentCoroutineContext().ensureActive()
        val run = generation; val current = checkNotNull(api)
        val result = withContext(Dispatchers.IO) { current.json(method, path, bearer, body) }
        currentCoroutineContext().ensureActive(); if (run != generation) throw CancellationException()
        return result
    }
    private fun pair() {
        val qr = try { PairQr.parse(qrField.text.toString()) } catch (_: Exception) { message.text = "QR non valido: usa quello mostrato dal PC sulla stessa rete locale."; return }
        disconnect(); api = ApiClient(qr); qrField.text.clear()
        work {
            val response = request("POST", "/v1/pair", null, JSONObject().put("secret", qr.secret).put("name", "Android"))
            pairId = checkNotNull(api).id(response.getString("id")); pollToken = response.getString("pollToken")
            val code = response.getString("code"); require(code.matches(Regex("[0-9]{6}"))) { "Codice di confronto non valido" }
            message.text = "Codice: $code\nConfrontalo sul PC e approva solo se coincide."
            busy = false; refresh(); pollLoop()
        }
    }
    private fun poll() {
        if (!foreground || busy || job?.isActive == true || api == null) return
        val run = generation
        job = lifecycleScope.launch {
            try { pollLoop() } catch (_: CancellationException) {} catch (e: Exception) { if (run == generation) message.text = "Connessione interrotta: ${e.message}" }
            finally { if (run == generation) job = null }
        }
    }
    private suspend fun pollLoop() {
        while (foreground) {
            if (token == null) {
                val id = pairId ?: return
                val result = request("GET", "/v1/pair/$id", pollToken)
                when (result.getString("status")) {
                    "approved" -> { token = result.getString("token"); pairId = null; pollToken = null; message.text = "PC associato" }
                    "denied", "expired" -> { disconnect(); message.text = "Associazione rifiutata o scaduta. Genera un nuovo QR."; return }
                    "pending" -> Unit
                    else -> error("Stato associazione non valido")
                }
            } else {
                val state = request("GET", "/v1/state"); check(state.getBoolean("paired"))
                outgoing = state.optJSONObject("outgoing")?.also { validateOffer(it) }
                if (outgoing != null && ackId == null) message.text = "Dal PC: ${safeName(outgoing!!.getString("name"))} (${outgoing!!.getLong("size")} byte)"
            }
            refresh(); delay(1500)
        }
    }
    private fun validateOffer(offer: JSONObject) {
        checkNotNull(api).id(offer.getString("id")); require(offer.getLong("size") in 0..MAX_FILE)
        require(offer.getString("name").length in 1..255)
    }
    private fun safeName(name: String) = name.substringAfterLast('/').substringAfterLast('\\').filter { !it.isISOControl() }.take(180).ifBlank { "file" }
    private fun send(uri: Uri) = work {
        val run = generation
        val info = withContext(Dispatchers.IO) {
            contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
                require(c.moveToFirst()); val sizeIndex = c.getColumnIndexOrThrow(OpenableColumns.SIZE)
                require(!c.isNull(sizeIndex)) { "Dimensione file sconosciuta" }
                safeName(c.getString(c.getColumnIndexOrThrow(OpenableColumns.DISPLAY_NAME))) to c.getLong(sizeIndex)
            } ?: error("File non disponibile")
        }
        require(info.second in 0..MAX_FILE) { "Limite 256 MiB" }
        transferId = checkNotNull(api).id(request("POST", "/v1/offers", body = JSONObject().put("name", info.first).put("size", info.second)).getString("id"))
        message.text = "Attendi l'approvazione del file sul PC"
        while (true) {
            val incoming = request("GET", "/v1/state").optJSONObject("incoming") ?: error("Offerta annullata")
            check(incoming.getString("id") == transferId)
            when (incoming.getString("status")) { "accepted" -> break; "pending" -> delay(1500); else -> error("File rifiutato o annullato") }
        }
        val current = checkNotNull(api); val id = checkNotNull(transferId); val bearer = checkNotNull(token)
        withContext(Dispatchers.IO) {
            contentResolver.openInputStream(uri)?.use { input ->
                current.upload(id, bearer, info.second, input) { n -> runOnUiThread { if (run == generation) message.text = "Invio: $n / ${info.second} byte" } }
            } ?: error("File non leggibile")
        }
        transferId = null; message.text = "File ricevuto dal PC"
    }
    private fun receive(uri: Uri, offer: JSONObject) = work {
        val run = generation
        validateOffer(offer); transferId = offer.getString("id"); var saved = false
        val current = checkNotNull(api); val id = checkNotNull(transferId); val bearer = checkNotNull(token)
        try {
            withContext(Dispatchers.IO) {
                contentResolver.openOutputStream(uri, "w")?.use { output ->
                    current.download(id, bearer, offer.getLong("size"), output) { n -> runOnUiThread { if (run == generation) message.text = "Ricezione: $n / ${offer.getLong("size")} byte" } }
                } ?: error("Destinazione non scrivibile")
            }
            saved = true; ackId = transferId; transferId = null
            request("POST", "/v1/outgoing/${ackId}/done"); ackId = null; outgoing = null
            message.text = "File salvato"
        } finally {
            if (!saved) withContext(NonCancellable + Dispatchers.IO) { runCatching { DocumentsContract.deleteDocument(contentResolver, uri) } }
        }
    }
    private fun acknowledge() = work {
        val id = ackId ?: return@work
        request("POST", "/v1/outgoing/$id/done"); ackId = null; outgoing = null
        message.text = "Salvataggio confermato al PC"
    }
    private fun cancelTransfer() {
        val id = transferId ?: outgoing?.optString("id") ?: return
        work { request("DELETE", "/v1/transfers/${checkNotNull(api).id(id)}"); transferId = null; outgoing = null; message.text = "Trasferimento annullato" }
    }
    private fun disconnect() { generation++; job?.cancel(); api?.abort(); job = null; pendingAction = null; api = null; token = null; pairId = null; pollToken = null; outgoing = null; transferId = null; ackId = null; busy = false; if (::pairButton.isInitialized) refresh() }
}
