// Original NotchLite companion code. SPDX-License-Identifier: Apache-2.0
package it.notchlite.companion

import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import org.json.JSONObject
import java.io.InputStream
import java.io.OutputStream
import java.net.URL
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.cert.CertificateException
import java.security.cert.X509Certificate
import java.util.concurrent.atomic.AtomicReference
import javax.net.ssl.HttpsURLConnection
import javax.net.ssl.SSLContext
import javax.net.ssl.X509TrustManager

const val MAX_FILE = 256L * 1024 * 1024

class ApiClient(private val qr: PairQr) {
    private val active = AtomicReference<HttpsURLConnection?>()
    private val tls = SSLContext.getInstance("TLS").apply {
        val expected = qr.pin.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
        init(null, arrayOf(object : X509TrustManager {
            override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
            override fun checkClientTrusted(chain: Array<X509Certificate>, authType: String) { throw CertificateException("Client TLS non supportato") }
            override fun checkServerTrusted(chain: Array<X509Certificate>, authType: String) {
                val leaf = chain.firstOrNull() ?: throw CertificateException("Certificato mancante")
                if (!MessageDigest.isEqual(expected, MessageDigest.getInstance("SHA-256").digest(leaf.encoded))) throw CertificateException("Pin TLS diverso dal QR")
                leaf.checkValidity()
            }
        }), SecureRandom())
    }
    fun abort() { active.getAndSet(null)?.disconnect() }
    fun id(value: String): String { require(value.matches(Regex("[A-Za-z0-9_-]{1,128}"))); return value }
    private fun connection(method: String, path: String, bearer: String?): HttpsURLConnection {
        require(path.startsWith("/v1/") && !path.contains('?') && !path.contains('#'))
        val c = URL("https", qr.host, qr.port, path).openConnection() as HttpsURLConnection
        c.sslSocketFactory = tls.socketFactory
        // Keep the platform hostname verifier: the pinned certificate must also have the LAN IP SAN.
        c.instanceFollowRedirects = false; c.connectTimeout = 10_000; c.readTimeout = 15_000; c.requestMethod = method
        c.useCaches = false
        if (bearer != null) { require(bearer.matches(Regex("[A-Za-z0-9_-]{1,256}"))); c.setRequestProperty("Authorization", "Bearer $bearer") }
        check(active.compareAndSet(null, c)) { "Richiesta già in corso" }
        return c
    }
    suspend fun json(method: String, path: String, bearer: String? = null, body: JSONObject? = null): JSONObject {
        currentCoroutineContext().ensureActive()
        val c = connection(method, path, bearer)
        try {
            if (body == null && method == "POST") { c.doOutput = true; c.setFixedLengthStreamingMode(0); c.connect(); c.outputStream.close() }
            if (body != null) {
                val bytes = body.toString().toByteArray(Charsets.UTF_8)
                c.doOutput = true; c.setRequestProperty("Content-Type", "application/json"); c.setFixedLengthStreamingMode(bytes.size)
                c.connect() // Completes TLS pin and hostname validation before writing secrets.
                c.outputStream.use { it.write(bytes) }
            }
            val status = c.responseCode
            check(status == 200) { "PC: HTTP $status" }
            require(c.contentLengthLong <= 65536) { "Risposta troppo grande" }
            val out = java.io.ByteArrayOutputStream()
            c.inputStream.use { input ->
                val buffer = ByteArray(4096)
                while (true) {
                    currentCoroutineContext().ensureActive()
                    val n = input.read(buffer); if (n < 0) break
                    require(out.size() + n <= 65536) { "Risposta troppo grande" }; out.write(buffer, 0, n)
                }
            }
            return if (out.size() == 0) JSONObject() else JSONObject(out.toString("UTF-8"))
        } finally { active.compareAndSet(c, null); c.disconnect() }
    }
    suspend fun upload(id: String, token: String, size: Long, input: InputStream, progress: (Long) -> Unit) {
        currentCoroutineContext().ensureActive()
        require(size in 0..MAX_FILE)
        val c = connection("PUT", "/v1/incoming/${id(id)}", token)
        try {
            c.doOutput = true; c.setRequestProperty("Content-Type", "application/octet-stream"); c.setFixedLengthStreamingMode(size); c.connect()
            c.outputStream.use { copyExact(input, it, size, progress) }
            check(c.responseCode == 200) { "Upload non confermato: HTTP ${c.responseCode}" }
        } finally { active.compareAndSet(c, null); c.disconnect() }
    }
    suspend fun download(id: String, token: String, size: Long, output: OutputStream, progress: (Long) -> Unit) {
        currentCoroutineContext().ensureActive()
        require(size in 0..MAX_FILE)
        val c = connection("GET", "/v1/outgoing/${id(id)}", token)
        try {
            check(c.responseCode == 200) { "Download: HTTP ${c.responseCode}" }
            require(c.contentLengthLong == size) { "Dimensione file diversa dall'offerta" }
            c.inputStream.use { copyExact(it, output, size, progress) }; output.flush()
        } finally { active.compareAndSet(c, null); c.disconnect() }
    }
    private suspend fun copyExact(input: InputStream, output: OutputStream, size: Long, progress: (Long) -> Unit) {
        var copied = 0L; var last = 0L; val buffer = ByteArray(65536)
        while (copied < size) {
            currentCoroutineContext().ensureActive()
            val n = input.read(buffer, 0, minOf(buffer.size.toLong(), size - copied).toInt())
            check(n > 0) { "File incompleto" }; output.write(buffer, 0, n); copied += n
            val now = System.nanoTime(); if (now - last > 100_000_000) { progress(copied); last = now }
        }
        require(input.read() == -1) { "Il file è cambiato durante il trasferimento" }; progress(copied)
    }
}
