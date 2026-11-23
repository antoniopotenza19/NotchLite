package it.notchlite.companion

import org.junit.Assert.*
import org.junit.Test

class PairQrTest {
    private val base = "notchlite://pair?v=1&host=192.168.1.2&port=43123&pin=${"0".repeat(64)}&secret=${"1".repeat(64)}"
    private fun rejects(value: String) { assertThrows(Exception::class.java) { PairQr.parse(value) } }
    @Test fun privateRanges() {
        for (host in listOf("10.0.0.1", "172.16.0.1", "172.31.255.254", "192.168.1.2"))
            assertEquals(host, PairQr.parse(base.replace("192.168.1.2", host)).host)
    }
    @Test fun rejectUntrustedEndpoints() {
        for (host in listOf("example.org", "127.0.0.1", "8.8.8.8", "172.32.0.1", "224.0.0.1", "192.168.001.2", "192.168.1.999", "192%2E168.1.2"))
            rejects(base.replace("192.168.1.2", host))
        rejects(base.replace("//pair?", "//user@pair?")); rejects(base.replace("//pair?", "//pair/path?")); rejects("$base#fragment")
    }
    @Test fun rejectAmbiguityAndMalformedCredentials() {
        rejects("$base&host=10.0.0.1"); rejects("$base&extra=x")
        rejects(base.replace("pin=${"0".repeat(64)}", "pin=${"g".repeat(64)}"))
        rejects(base.replace("43123", "0")); rejects(base.replace("43123", "65536")); rejects(base.replace("v=1", "v=2"))
        rejects(base + "x".repeat(2048))
    }
}
