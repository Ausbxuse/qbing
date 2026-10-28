package lab.cube.core

// GAN Gen4 protocol port, derived from gan-web-bluetooth 3.0.2 (MIT).
// Attribution and upstream license are bundled in composeResources/files.
sealed interface CubePacket {
    data class State(val serial: Int,val facelets: String): CubePacket
    data class Move(val serial: Int,val move: String,val cubeMs: Double?,val localMs: Double?,val recovered: Boolean=false): CubePacket
    data class Battery(val percentage: Int): CubePacket
    /** Normalized renderer-space quaternion, x/y/z/w. */
    data class Orientation(val quaternion: List<Float>): CubePacket
    data class Hardware(val name: String): CubePacket
    data class Fault(val reason: String): CubePacket
    data object Disconnect: CubePacket
}

class GanCipher(mac: String) {
    private val baseKey=intArrayOf(0x01,0x02,0x42,0x28,0x31,0x91,0x16,0x07,0x20,0x05,0x18,0x54,0x42,0x11,0x12,0x53)
    private val baseIv=intArrayOf(0x11,0x03,0x32,0x28,0x21,0x01,0x76,0x27,0x20,0x95,0x78,0x14,0x32,0x12,0x02,0x43)
    private val key: ByteArray
    private val iv: ByteArray
    init {
        require(Regex("([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}").matches(mac)){"Enter a valid cube MAC address."}
        val salt=mac.split(':').map{it.toInt(16)}.reversed()
        key=ByteArray(16){if(it<6)((baseKey[it]+salt[it])%255).toByte() else baseKey[it].toByte()}
        iv=ByteArray(16){if(it<6)((baseIv[it]+salt[it])%255).toByte() else baseIv[it].toByte()}
    }
    fun encrypt(input: ByteArray)=transform(input,true)
    fun decrypt(input: ByteArray)=transform(input,false)
    private fun transform(input: ByteArray,encrypt: Boolean): ByteArray {
        require(input.size in 16..128){"Invalid GAN packet size."}
        val result=input.copyOf();val offsets=if(input.size==16)listOf(0) else if(encrypt)listOf(0,input.size-16) else listOf(input.size-16,0)
        for(offset in offsets)aesBlock(result.copyOfRange(offset,offset+16),key,iv,encrypt).copyInto(result,offset)
        return result
    }
}
object GanUuids {
    const val service="00000010-0000-fff7-fff6-fff5fff4fff0"
    const val command="0000fff5-0000-1000-8000-00805f9b34fb"
    const val notify="0000fff6-0000-1000-8000-00805f9b34fb"
    const val cccd="00002902-0000-1000-8000-00805f9b34fb"
}
class GanProtocol(mac: String,private val emit: (CubePacket)->Unit) {
    private val cipher=GanCipher(mac)
    fun command(name: String): ByteArray {
        val bytes=when(name){"state"->intArrayOf(0xdd,4,0,0xed,0,0);"battery"->intArrayOf(0xdd,4,0,0xef,0,0);"hardware"->intArrayOf(0xdf,3,0,0,0);"reset"->intArrayOf(0xd2,0x0d,5,0x39,0x77,0,0,1,0x23,0x45,0x67,0x89,0xab,0,0,0);else->error("Unknown GAN command")}
        return cipher.encrypt(ByteArray(20){if(it<bytes.size)bytes[it].toByte() else 0})
    }
    fun history(serial: Int,count: Int): ByteArray? {
        val start=if(serial%2==0)(serial-1)and 255 else serial and 255
        val size=minOf(if(count%2==0)count else count+1,start+1,16)
        if(size<=0)return null
        // Firmware returns fabricated moves if a history request crosses 255→0.
        return cipher.encrypt(ByteArray(20).apply{this[0]=0xd1.toByte();this[1]=4;this[2]=start.toByte();this[4]=size.toByte()})
    }
    fun accept(encrypted: ByteArray,receivedAt: Double=monotonicMs()) {
        try {decode(cipher.decrypt(encrypted),receivedAt)}catch(e: Exception){emit(CubePacket.Fault("Invalid cube packet. Check its MAC address and reconnect."))}
    }
    fun decode(data: ByteArray,receivedAt: Double) {
        fun byte(i:Int):Int{require(i in data.indices);return data[i].toInt()and 255}
        fun bits(offset:Int,count:Int):Int {require(offset+count<=data.size*8);var value=0;for(i in offset until offset+count)value=(value shl 1)or ((byte(i/8) shr (7-i%8))and 1);return value}
        fun le(offset:Int,count:Int):Long {var value=0L;for(i in 0 until count)value=value or (byte(offset+i).toLong() shl (8*i));return value}
        val type=byte(0);val length=byte(1)
        require(length<=data.size-2)
        when(type) {
            0x01->{val serial=le(6,2).toInt();val direction=bits(64,2);val face=mapOf(2 to 'U',32 to 'R',8 to 'F',1 to 'D',16 to 'L',4 to 'B')[bits(66,6)]?:error("Invalid face");require(direction in 0..1);emit(CubePacket.Move(serial,"$face${if(direction==1)"'" else ""}",le(2,4).toDouble(),receivedAt))}
            0xd1->{val start=byte(2);val count=(length-1)*2;require(count in 0..32);for(i in 0 until count){val face=bits(24+4*i,3);val direction=bits(27+4*i,1);require(face in 0..5);emit(CubePacket.Move((start-i)and 255,"${"DUBFLR"[face]}${if(direction==1)"'" else ""}",null,null,true))}}
            0xed->{val serial=le(2,2).toInt();val cp=(0..6).map{bits(32+it*3,3)}.toMutableList();val co=(0..6).map{bits(53+it*2,2)}.toMutableList();val ep=(0..10).map{bits(69+it*4,4)}.toMutableList();val eo=(0..10).map{bits(113+it,1)}.toMutableList();cp.add(28-cp.sum());co.add((3-co.sum()%3)%3);ep.add(66-ep.sum());eo.add((2-eo.sum()%2)%2);emit(CubePacket.State(serial,Cube.fromCubies(cp,co,ep,eo)))}
            0xec->if(length>=8){
                fun component(offset:Int):Float {val n=(byte(offset) shl 8)or byte(offset+1);return (if((n and 0x8000)==0)1f else -1f)*(n and 0x7fff)/32767f}
                val q=listOf(component(4),component(8),-component(6),component(2))
                val norm=kotlin.math.sqrt(q.sumOf{(it*it).toDouble()}).toFloat()
                if(norm>0.0001f)emit(CubePacket.Orientation(q.map{it/norm}))
            }
            0xef->emit(CubePacket.Battery(minOf(byte(1+length),100)))
            0xfc->emit(CubePacket.Hardware((0 until length-1).map{byte(3+it).toChar()}.joinToString("")))
            0xea->emit(CubePacket.Disconnect)
        }
    }
}

class MoveStream(
    private val onState: (String)->Unit,
    private val onMove: (CubePacket.Move,String)->Unit,
    private val onFault: (String)->Unit,
    private val requestHistory: (Int,Int)->Unit,
    private val onVerified: (Int)->Unit = {}
) {
    var state: String?=null;private set
    var serial: Int?=null;private set
    private val pending=mutableMapOf<Int,CubePacket.Move>()
    private val snapshots=mutableMapOf<Int,CubePacket.State>()
    private val recent=ArrayDeque<Pair<Int,String>>()
    private var waiting: Double?=null
    private var lastLive: Double?=null
    private var historyRequest: Triple<Int,Int,Double>?=null
    fun reset(){state=null;serial=null;pending.clear();snapshots.clear();recent.clear();waiting=null;lastLive=null;historyRequest=null}
    private fun diff(a:Int,b:Int)=(a-b+256)and 255
    fun accept(packet: CubePacket,now: Double=monotonicMs()) {
        when(packet){
            is CubePacket.State->{
                if(!Cube.valid(packet.facelets)){reset();onFault("invalid-state");return}
                val s=packet.serial and 255;val current=serial
                if(current==null){state=packet.facelets;serial=s;recent.addLast(s to packet.facelets);onState(packet.facelets);return}
                val delta=diff(s,current)
                if(delta==0){if(state!=packet.facelets){onFault("state-mismatch");reset();state=packet.facelets;serial=s;recent.addLast(s to packet.facelets);onState(packet.facelets)}else onVerified(s)}
                else if(delta<128){
                    snapshots[s]=packet;if(waiting==null)waiting=now
                    // A state packet can overtake its live move notification.
                    // Match csTimer / gan-web-bluetooth's 500 ms quiet interval.
                    poll(now)
                }
                else recent.firstOrNull{it.first==s}?.let{if(it.second==packet.facelets)onVerified(s) else {reset();onFault("state-mismatch")}}
            }
            is CubePacket.Move->{
                val current=serial?:return;val s=packet.serial and 255;val delta=diff(s,current)
                if(delta==0||delta>=128)return
                if(packet.recovered){
                    // GAN pads history to pairs; only fill an evidenced gap.
                    val bound=(pending.keys+snapshots.keys).maxOfOrNull{diff(it,current)}?:0
                    if(delta>bound)return
                    // Preserve the hardware timestamp of an existing live move.
                    if(!pending.containsKey(s))pending[s]=packet
                }else{lastLive=now;pending[s]=packet}
                if(delta>1){if(waiting==null)waiting=now;if(!packet.recovered)poll(now)}
                while(pending.containsKey((serial!!+1)and 255)) {
                    val next=(serial!!+1)and 255;val move=pending.remove(next)!!
                    val updated=Cube.turn(state!!,move.move);state=updated;serial=next;recent.addLast(next to updated);while(recent.size>96)recent.removeFirst();onMove(move,updated)
                    val snapshot=snapshots.remove(next)
                    if(snapshot!=null){if(snapshot.facelets!=updated){onFault("state-mismatch");reset();state=snapshot.facelets;serial=next;recent.addLast(next to snapshot.facelets);onState(snapshot.facelets);break}else onVerified(next)}
                }
                if(pending.isEmpty()&&snapshots.isEmpty()){waiting=null;historyRequest=null}
                if(pending.size>16){reset();onFault("missing-moves")}
            }
            else->Unit
        }
    }
    fun poll(now:Double=monotonicMs()) {
        val current=serial?:return
        val gap=pending.keys.map{it to diff(it,current)}.filter{it.second in 2..127}.minByOrNull{it.second}
        val request=if(gap!=null)gap else {
            val quietSince=lastLive?:waiting?:return
            if(now-quietSince<500)return
            snapshots.keys.mapNotNull{s->
                val count=diff(s,current)
                // Serial zero snapshots are ambiguous at the firmware wrap.
                // +1 includes an even final move in GAN's odd-aligned window.
                if(s!=0&&count in 1..16){
                    if(count==16&&s%2==0)s to count else ((s+1)and 255) to (count+1)
                }else null
            }.minByOrNull{it.second}
        }?:return
        val previous=historyRequest
        if(previous!=null&&previous.first==request.first&&previous.second==request.second&&now-previous.third<200)return
        historyRequest=Triple(request.first,request.second,now)
        requestHistory(request.first,request.second)
    }
    fun expire(now:Double=monotonicMs()):Boolean {
        if(waiting==null||now-waiting!!<1200){poll(now);return false}
        val newest=snapshots.values.maxByOrNull{diff(it.serial and 255,serial?:0)}
        reset();onFault("missing-moves");if(newest!=null)accept(newest,now);return true
    }
}
