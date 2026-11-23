// Original NotchLite companion code. SPDX-License-Identifier: Apache-2.0
package it.notchlite.companion

import java.net.URI

data class PairQr(val host: String, val port: Int, val pin: String, val secret: String) {
    companion object {
        fun parse(raw: String): PairQr {
            require(raw.length <= 2048 && raw.none { it.isWhitespace() }) { "QR non valido" }
            val uri = URI(raw)
            require(uri.scheme == "notchlite" && uri.rawAuthority == "pair" && uri.rawPath.isNullOrEmpty() && uri.rawFragment == null)
            val parts = (uri.rawQuery ?: error("QR incompleto")).split('&')
            val map = linkedMapOf<String, String>()
            for (part in parts) {
                val kv = part.split('=')
                require(kv.size == 2 && map.put(kv[0], kv[1]) == null) { "Parametri QR duplicati" }
            }
            require(map.keys == setOf("v", "host", "port", "pin", "secret") && map["v"] == "1")
            val host = map.getValue("host")
            val octets = host.split('.')
            require(octets.size == 4 && octets.all { it.matches(Regex("0|[1-9][0-9]{0,2}")) && it.toInt() <= 255 })
            val n = octets.map { it.toInt() }
            require(n[0] == 10 || n[0] == 172 && n[1] in 16..31 || n[0] == 192 && n[1] == 168) { "Serve un indirizzo LAN privato" }
            val portText = map.getValue("port")
            require(portText.matches(Regex("[1-9][0-9]{0,4}")))
            val port = portText.toInt(); require(port in 1..65535)
            for (key in listOf("pin", "secret")) require(map.getValue(key).matches(Regex("[0-9a-fA-F]{64}")))
            return PairQr(host, port, map.getValue("pin").lowercase(), map.getValue("secret"))
        }
    }
}
