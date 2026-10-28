package lab.cube.core

import java.net.HttpURLConnection
import java.net.URI
import java.time.Instant
import java.util.UUID
import javax.crypto.Cipher
import javax.crypto.spec.IvParameterSpec
import javax.crypto.spec.SecretKeySpec
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

actual fun monotonicMs() = System.nanoTime()/1_000_000.0
actual fun nowIso(): String = Instant.now().toString()
actual fun newId(): String = UUID.randomUUID().toString()
actual fun aesBlock(data: ByteArray,key: ByteArray,iv: ByteArray,encrypt: Boolean): ByteArray {
    require(data.size==16)
    return Cipher.getInstance("AES/CBC/NoPadding").run {
        init(if(encrypt)Cipher.ENCRYPT_MODE else Cipher.DECRYPT_MODE,SecretKeySpec(key,"AES"),IvParameterSpec(iv));doFinal(data)
    }
}
actual suspend fun httpRequest(url: String,method: String,token: String,body: String?): String = withContext(Dispatchers.IO) {
    val uri=URI(url)
    require(uri.scheme=="https"||(uri.scheme=="http"&&uri.host in listOf("localhost","127.0.0.1"))){"Use HTTPS for the server connection."}
    val connection=uri.toURL().openConnection() as HttpURLConnection
    connection.requestMethod=method;connection.connectTimeout=10000;connection.readTimeout=20000;connection.instanceFollowRedirects=false
    connection.setRequestProperty("Accept","application/json")
    if(token.isNotBlank())connection.setRequestProperty("Authorization","Bearer $token")
    if(body!=null){connection.doOutput=true;connection.setRequestProperty("Content-Type","application/json");connection.outputStream.use{it.write(body.toByteArray(Charsets.UTF_8))}}
    try {val code=connection.responseCode;val stream=if(code in 200..299)connection.inputStream else connection.errorStream;val result=stream?.bufferedReader()?.use{it.readText()}?:"";check(code in 200..299){if(code==401)"Server access key required. Open settings." else "Server error $code: ${result.take(180)}"};result}finally{connection.disconnect()}
}

actual fun isoFromEpoch(seconds:Long):String=java.time.Instant.ofEpochSecond(seconds).toString()
actual fun nativeCreatedLabel(iso:String):String=runCatching{java.time.Instant.parse(iso).atZone(java.time.ZoneId.systemDefault()).format(java.time.format.DateTimeFormatter.ofPattern("MMM d, hh:mm a",java.util.Locale.ENGLISH))}.getOrDefault("")
