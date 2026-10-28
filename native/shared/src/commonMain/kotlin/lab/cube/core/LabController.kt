package lab.cube.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.*
import lab.cube.resources.Res
import org.jetbrains.compose.resources.ExperimentalResourceApi

data class PracticeTarget(val position: CasePosition,val caseId: String?=null,val algorithmId: String?=null,val stage: String="solve")
data class VisualTurn(val id:Long,val state:String,val move:String?,val cubeMs:Double?=null,val recovered:Boolean=false)

/** A device can briefly report a goal within a fast same-axis physical turn or slice burst. */
internal class GoalSettle {
    data class Candidate(val packet:CubePacket.Move,val elapsed:Double,val offeredAt:Double,var verified:Boolean=false)
    var candidate:Candidate?=null;private set
    fun offer(packet:CubePacket.Move,elapsed:Double,now:Double):Candidate=Candidate(packet,elapsed,now).also{candidate=it}
    fun ready(now:Double)=candidate?.let{now-it.offeredAt>=120.0}==true
    fun verify(serial:Int){candidate?.takeIf{(it.packet.serial and 255)==(serial and 255)}?.verified=true}
    fun clear(){candidate=null}
    fun take():Candidate?=candidate.also{candidate=null}
    fun continuesWith(next:CubePacket.Move,now:Double):Boolean {
        val pending=candidate?:return false
        val first=pending.packet
        if(ready(now)||((next.serial-first.serial)and 255)!=1)return false
        val a=first.cubeMs?.takeIf{it.isFinite()}
        val b=next.cubeMs?.takeIf{it.isFinite()}
        // Recovery packets have no trustworthy event clock. A short local burst
        // can still mean the goal is transient; this never upgrades timing quality
        // or changes notation/animation inference, which require device timing.
        val gap=if(!first.recovered&&!next.recovered&&a!=null&&b!=null)b-a else now-pending.offeredAt
        if(gap !in 0.0..100.0)return false
        fun axis(move:String):Int?=when(move.firstOrNull()){'R','L'->0;'U','D'->1;'F','B'->2;else->null}
        val firstAxis=axis(first.move)?:return false
        // Two quarter-turn packets may be one physical R2. M2 packets may arrive
        // R R L' L', so require a quiet goal rather than stopping within that burst.
        return firstAxis==axis(next.move)
    }
}

/** Bounded retries tolerate one lost reply without leaving a permanent fake connection. */
internal class StateRequestSchedule {
    private var started:Double?=null
    private var retries=0
    fun begin(now:Double){if(started==null){started=now;retries=0}}
    fun clear(){started=null;retries=0}
    enum class Action { NONE, RETRY, TIMEOUT }
    fun poll(now:Double):Action {
        val elapsed=now-(started?:return Action.NONE)
        if(elapsed>=6000){clear();return Action.TIMEOUT}
        if(retries<2&&elapsed>=if(retries==0)1000 else 3000){retries++;return Action.RETRY}
        return Action.NONE
    }
}

class LabController(private val transport: BleTransport,private val storage: LocalStorage,val scope: CoroutineScope=CoroutineScope(SupervisorJob()+Dispatchers.Main)) {
    var catalog by mutableStateOf(Catalog(emptyList(),emptyMap()));private set
    var records by mutableStateOf(emptyList<AttemptRecord>());private set
    var performances by mutableStateOf(emptyList<CasePerformance>());private set
    var preferences by mutableStateOf(emptyMap<String,Preferences>());private set
    var settings by mutableStateOf(runCatching{WireJson.decodeFromString<ClientSettings>(storage.read("settings")?:"{}")}.getOrDefault(ClientSettings()));private set
    var graphMetric by mutableStateOf(runCatching{GraphMetric.valueOf(storage.read("graph-metric")?:"Time")}.getOrDefault(GraphMetric.Time));private set
    fun selectGraphMetric(value:GraphMetric){graphMetric=value;storage.write("graph-metric",value.name)}
    var page by mutableStateOf("Practice")
    var foreground by mutableStateOf(true)
    var openBluetoothSettings:()->Unit={}
    var connected by mutableStateOf(false);private set
    var adapterStatus by mutableStateOf("Bluetooth ready");private set
    var deviceName by mutableStateOf("");private set
    var battery by mutableStateOf<Int?>(null);private set
    var devices by mutableStateOf(emptyList<CubeDevice>());private set
    var showDevices by mutableStateOf(false)
    var scanning by mutableStateOf(false);private set
    var connectionBusy by mutableStateOf(false);private set
    var cubeState by mutableStateOf<String?>(null);private set
    var visualTurns by mutableStateOf(emptyList<VisualTurn>());private set
    private var visualId=0L
    var target by mutableStateOf<PracticeTarget?>(null);private set
    var recoveryMoves by mutableStateOf(emptyList<String>());private set
    var scrambleCheckpoint by mutableStateOf<String?>(null);private set
    var recoveryBusy by mutableStateOf(false);private set
    var recoveryError by mutableStateOf<String?>(null);private set
    var scrambleLoading by mutableStateOf(false);private set
    var orientation by mutableStateOf<List<Float>?>(null);private set
    private var gyroViewFrame:Pair<List<Float>,List<Float>>?=null
    fun captureGyroFrame(basis:List<Float>,pose:List<Float>){
        gyroViewFrame=basis.toList() to pose.toList()
        active?.captureViewFrame(basis,pose)
    }
    var gyroLive by mutableStateOf(false);private set
    private var orientationAt:Double?=null
    private var targetGeneration=0L
    private var recoveryRevision=0L
    private var recoveryJob:Job?=null
    var active by mutableStateOf<ActiveAttempt?>(null);private set
    var cueCountdown by mutableStateOf<Int?>(null);private set
    var awaitingCue by mutableStateOf(false);private set
    var elapsed by mutableStateOf(0.0);private set
    var moves by mutableStateOf(emptyList<MoveSample>());private set
    var latest by mutableStateOf<AttemptRecord?>(null);private set
    var review by mutableStateOf<AttemptRecord?>(null)
    var selectedCase by mutableStateOf<String?>(null)
    var selectedVariant by mutableStateOf(0)
    var caseStage by mutableStateOf("PLL")
    var search by mutableStateOf("")
    var message by mutableStateOf("")
    var errorMessage by mutableStateOf<String?>(null)
    var syncStatus by mutableStateOf("Connecting to milkyway");private set
    var syncBusy by mutableStateOf(false);private set
    var queueCount by mutableStateOf(0);private set
    private var pending=runCatching{WireJson.decodeFromString<List<AttemptRecord>>(storage.read("pending")?:"[]")}.getOrDefault(emptyList()).toMutableList()
    private var protocol:GanProtocol?=null
    private var connectionEpoch=0L
    private var connectionJob:Job?=null
    private val connectionMutex=Mutex()
    private val stateRequests=StateRequestSchedule()
    private var lastDraft=0.0
    private val commands=Channel<Pair<Long,ByteArray>>(Channel.UNLIMITED)
    private fun enqueue(bytes:ByteArray){commands.trySend(connectionEpoch to bytes)}
    private val syncMutex=Mutex()
    private var pendingVerification:Pair<String,Int>?=null
    private val goalSettle=GoalSettle()
    private var completionJob:Job?=null
    private var cueJob:Job?=null
    private var tick=0
    private val scrambleTracker=ScrambleTracker()
    var scrambleSpeed by mutableStateOf(ScrambleStats());private set
    private fun attachScramble(){active?.let{it.scrambleMoves=scrambleTracker.moves.toList();it.scrambleQuality=scrambleTracker.quality.toList()}}
    private val stream=MoveStream(
        onState={stateRequests.clear();message="";clearGuidance();cubeState=it;visualTurns=listOf(VisualTurn(++visualId,it,null));maybeReady();refreshGuidance()},
        onMove={packet,state->receiveMove(packet,state)},
        onFault={fault(it);scope.launch{yield();if(connected&&cubeState==null)beginStateRequest()}},
        requestHistory={serial,count->protocol?.history(serial,count)?.let{enqueue(it)}},
        onVerified={serial->goalSettle.verify(serial);val p=pendingVerification;if(p!=null&&p.second==serial){records.find{it.id==p.first}?.let{save(it.copy(quality=it.quality.filterNot{q->q=="unverified-final-state"}))};pendingVerification=null}}
    )
    init {
        queueCount=pending.size
        records=pending.toList()
        transport.onBytes={bytes->val epoch=connectionEpoch;val decoder=protocol;scope.launch{if(epoch==connectionEpoch&&(connected||connectionBusy))decoder?.accept(bytes)}}
        transport.onAdapter={powered,reason->scope.launch{adapterStatus=reason;if(!powered)endConnection("bluetooth-off",reason)}}
        transport.onDisconnected={reason->scope.launch{if(connected||connectionBusy)endConnection("connection-lost",reason)}}
        scope.launch {for((epoch,command) in commands){
            if(epoch!=connectionEpoch||!connected)continue
            try{transport.write(command)}catch(e:CancellationException){throw e}catch(e:Exception){
                if(epoch==connectionEpoch)endConnection("connection-lost",e.message?:"Bluetooth write failed. Reconnect your cube.")
            }
        }}
        scope.launch {
            try {
                loadCatalog()
                pending=pending.map{refreshAnalysis(it,catalog)}.toMutableList();persistPending()
                val cache=storage.read("cache")
                if(cache!=null){
                    val data=WireJson.decodeFromString<Bootstrap>(cache)
                    val local=records.associateBy{it.id}
            records=(data.records.filterNot{r->pending.any{it.id==r.id}}+pending).map{refreshAnalysis(retainReplayMetadata(it,local[it.id]),catalog)}.sortedBy{it.createdAt}
                    preferences=data.preferences
                    storage.write("cache",WireJson.encodeToString(data.copy(records=records)))
                }else records=pending.toList()
                storage.read("analytics")?.let{performances=WireJson.decodeFromString(it)}
                // A process exit during a solve is recoverable as an interrupted attempt.
                storage.read("draft")?.takeIf{it!=""}?.let{raw->runCatching{WireJson.decodeFromString<AttemptRecord>(raw)}.getOrNull()?.let{r->save(r.copy(status="interrupted",penalty="DNF",quality=r.quality+"app-closed"))};storage.write("draft","")}
                sync();if(target==null&&active==null)prepareNextScramble(false)
            }catch(e:Exception){message=e.message?:"Could not load the case library"}
        }
        scope.launch {while(isActive){delay(if(foreground)50 else 250);gyroLive=connected&&orientationAt?.let{monotonicMs()-it<2000}==true;active?.startedAt?.let{elapsed=goalSettle.candidate?.elapsed?:maxOf(active?.moves?.lastOrNull()?.t?:0.0,monotonicMs()-it)};if((target!=null||scrambleLoading)&&active==null&&!awaitingCue)scrambleSpeed=scrambleTracker.liveStats();tick++;pollStateRequest(monotonicMs());if(tick%5==0&&stream.expire())beginStateRequest();if(tick%600==0&&pending.isNotEmpty()&&!syncBusy)launch{sync()}}}
    }
    @OptIn(ExperimentalResourceApi::class)
    private suspend fun loadCatalog(){catalog=WireJson.decodeFromString(Res.readBytes("files/catalog.json").decodeToString())}
    private fun send(name:String){protocol?.let{enqueue(it.command(name))}}
    fun launch(block:suspend()->Unit){scope.launch{try{block()}catch(e:CancellationException){throw e}catch(e:Exception){message=e.message?:"Something went wrong";errorMessage=message}}}
    private fun persistPending(){queueCount=pending.size;storage.write("pending",WireJson.encodeToString(pending.toList()))}
    fun save(record:AttemptRecord){
        val updated=refreshAnalysis(record,catalog)
        pending.removeAll{it.id==updated.id};pending.add(updated);persistPending()
        records=(records.filterNot{it.id==updated.id}+updated).sortedBy{it.createdAt}
        if(latest?.id==updated.id)latest=updated
        if(review?.id==updated.id)review=updated
        launch{sync()}
    }
    suspend fun sync()=syncMutex.withLock {
        if(settings.token.isBlank()){syncStatus="Local";return@withLock}
        syncBusy=true
        try {
            for(r in pending.toList()) {
                val saved=refreshAnalysis(retainReplayMetadata(WireJson.decodeFromString<AttemptRecord>(request("records","POST",WireJson.encodeToString(r))),r),catalog)
                // Do not remove a newer revision queued while this request was in flight.
                pending.removeAll{it.id==r.id&&it==r};persistPending()
                if(pending.none{it.id==r.id}){records=(records.filterNot{it.id==r.id}+saved).sortedBy{it.createdAt};if(latest?.id==r.id)latest=saved}
            }
            val data=WireJson.decodeFromString<Bootstrap>(request("bootstrap"));preferences=data.preferences
            val local=records.associateBy{it.id}
            records=(data.records.filterNot{r->pending.any{it.id==r.id}}+pending).map{refreshAnalysis(retainReplayMetadata(it,local[it.id]),catalog)}.sortedBy{it.createdAt}
            latest=latest?.let{old->records.firstOrNull{it.id==old.id}?:old}
            review=review?.let{old->records.firstOrNull{it.id==old.id}?:old}
            storage.write("cache",WireJson.encodeToString(data.copy(records=records)))
            performances=WireJson.decodeFromString(request("algorithm-stats"));storage.write("analytics",WireJson.encodeToString(performances))
            syncStatus=if(pending.isEmpty())"Synced · milkyway" else "${pending.size} queued"
        }catch(e:Exception){syncStatus=if(pending.isEmpty())"Offline · cached data" else "Offline · ${pending.size} queued";if(e.message?.contains("access key")==true)message=e.message!!}
        finally{syncBusy=false}
    }
    private suspend fun request(route:String,method:String="GET",body:String?=null):String = if(route in listOf("scramble","recovery","cross","native-tools")) localSolver(route,body) else httpRequest(settings.endpoint.trimEnd('/')+"/"+route,method,settings.token,body)
    fun saveSettings(value:ClientSettings){
        check(active==null){"Finish or cancel the attempt first."}
        val reset=target==null||value.cross!=settings.cross||value.session!=settings.session
        settings=value;storage.write("settings",WireJson.encodeToString(value))
        if(reset){target=null;scrambleCheckpoint=null;targetGeneration++;scrambleLoading=false;clearGuidance();awaitingCue=false}
        if(target==null){val generation=targetGeneration;scope.launch{yield();if(generation==targetGeneration&&target==null&&active==null)prepareNextScramble(false)}}
        launch{sync()};message="Settings saved"
    }
    fun scan(){launch{scanning=true;showDevices=true;try{devices=transport.scan()}finally{scanning=false}}}
    fun connect(device:CubeDevice){
        if(connectionBusy||connected)return
        val epoch=++connectionEpoch
        connectionBusy=true;showDevices=false;message="Connecting to ${device.name}"
        stream.reset();stateRequests.clear();cubeState=null;battery=null;orientation=null;gyroViewFrame=null;orientationAt=null;gyroLive=false
        connectionJob=scope.launch {
            try {
                protocol=GanProtocol(device.address){packet->
                    if(epoch==connectionEpoch&&(connected||connectionBusy))when(packet){
                        is CubePacket.State,is CubePacket.Move->stream.accept(packet)
                        is CubePacket.Battery->battery=packet.percentage
                        is CubePacket.Orientation->{orientation=packet.quaternion;orientationAt=monotonicMs();gyroLive=true;gyroViewFrame?.let{(basis,pose)->active?.captureViewFrame(basis,pose,orientationAt!!)};active?.orientation(packet.quaternion,orientationAt!!)}
                        is CubePacket.Hardware->Unit
                        is CubePacket.Fault->{fault("invalid-state");stream.reset();beginStateRequest();message=packet.reason}
                        CubePacket.Disconnect->endConnection("connection-lost","Cube disconnected. Reconnect before the next attempt.")
                    }
                }
                connectionMutex.withLock{transport.connect(device)}
                if(epoch!=connectionEpoch)return@launch
                connected=true;deviceName=device.name;settings=settings.copy(mac=device.address)
                storage.write("settings",WireJson.encodeToString(settings))
                beginStateRequest();send("battery");send("hardware")
                if(cubeState==null)message="Connected. Reading cube state…"
            }catch(e:CancellationException){throw e}
            catch(e:Exception){if(epoch==connectionEpoch)endConnection("connection-lost",e.message?:"Could not connect to the cube.")}
            finally{if(epoch==connectionEpoch){connectionBusy=false;connectionJob=null}}
        }
    }
    private fun endConnection(faultReason:String,reason:String){
        val epoch=++connectionEpoch
        connectionJob?.cancel();connectionJob=null
        connected=false;connectionBusy=false;protocol=null;stateRequests.clear();stream.reset();fault(faultReason);message=reason
        scope.launch{connectionMutex.withLock{if(epoch==connectionEpoch)runCatching{transport.disconnect()}}}
    }
    private fun beginStateRequest(){
        if(!connected)return
        if(cubeState==null)stateRequests.begin(monotonicMs());send("state")
    }
    internal fun pollStateRequest(now:Double){
        if(!connected)return
        when(stateRequests.poll(now)){
            StateRequestSchedule.Action.RETRY->send("state")
            StateRequestSchedule.Action.TIMEOUT->endConnection("state-timeout","Cube did not return a valid state. Wake it and reconnect.")
            StateRequestSchedule.Action.NONE->Unit
        }
    }
    fun connectAddress(){val address=settings.mac.trim();if(address.isBlank())scan() else connect(CubeDevice(address,"GAN cube"))}
    fun disconnect(){endConnection("connection-lost","Disconnected")}
    fun requestState(){if(active!=null){message="Finish or cancel the attempt first.";return};if(cubeState==null)stream.reset();beginStateRequest()}
    fun calibrate(){if(active!=null){message="Finish or cancel the attempt first.";return};stream.reset();cubeState=null;target=null;scrambleCheckpoint=null;targetGeneration++;scrambleLoading=false;clearGuidance();awaitingCue=false;send("reset");beginStateRequest()}
    fun prepareCase(id:String,variant:Int=0){
        cancel();scrambleTracker.reset();scrambleSpeed=ScrambleStats();val item=catalog.cases.firstOrNull{it.id==id}?:return
        val alg=item.variants.getOrElse(variant){item.variants.first()};selectedCase=id;selectedVariant=variant
        target=PracticeTarget(alg.positions.getValue(settings.cross),id,alg.id,item.stage);page="Practice";moves=emptyList();maybeReady();refreshGuidance()
    }
    fun prepareSequence(sequence:List<String>){
        check(active==null){"Finish the attempt first."}
        val state=Cube.apply(Cube.solved,sequence)
        clearGuidance();targetGeneration++;scrambleCheckpoint=null;scrambleLoading=false;scrambleTracker.reset();scrambleSpeed=ScrambleStats()
        target=PracticeTarget(CasePosition(state,Cube.inverse(sequence),sequence));moves=emptyList();maybeReady();refreshGuidance()
    }
    fun prepareScramble()=prepareNextScramble(true)
    private fun prepareNextScramble(showPractice:Boolean){
        if(active!=null){message="Finish or cancel the attempt first.";return}
        val generation=++targetGeneration
        clearGuidance();target=null;scrambleCheckpoint=null;scrambleLoading=true;scrambleTracker.reset();scrambleSpeed=ScrambleStats()
        launch {
            try {
                val json=WireJson.parseToJsonElement(request("scramble")).jsonObject
                val sequence=json.getValue("scramble").jsonPrimitive.content.split(Regex("\\s+")).filter{it.isNotBlank()}
                if(generation!=targetGeneration||active!=null)return@launch
                target=PracticeTarget(CasePosition(Cube.apply(Cube.solved,sequence),Cube.inverse(sequence),sequence))
                moves=emptyList();if(showPractice)page="Practice";maybeReady();refreshGuidance()
            }catch(e:CancellationException){throw e}
            catch(e:Exception){if(generation==targetGeneration)message=e.message?:"Could not prepare a scramble"}
            finally{if(generation==targetGeneration)scrambleLoading=false}
        }
    }
    fun armCurrent(){val state=cubeState;if(state==null||!connected){message="Connect your cube first.";return};if(state==Cube.solved){message="Scramble the physical cube first.";return};cancel();target=null;active=ActiveAttempt(state,"solve","solve",settings.cross,settings.session);latest=null;moves=emptyList();elapsed=0.0;message="Armed. First turn starts the timer."}
    private fun maybeReady(){val t=target?:return;if(active!=null||cubeState!=t.position.state)return
        scrambleTracker.freeze();scrambleSpeed=scrambleTracker.stats()
        clearGuidance()
        if(t.caseId!=null){awaitingCue=true;message="Setup matched. Start the cue when ready."}
        else {active=ActiveAttempt(t.position.state,"solve","solve",settings.cross,settings.session,scramble=t.position.setup.joinToString(" "));attachScramble();message="Scramble matched. Turn to start."}
    }
    fun startCue(){
        if(!awaitingCue||cubeState!=target?.position?.state||cueJob?.isActive==true)return
        cueJob=scope.launch {for(n in 3 downTo 1){cueCountdown=n;delay(650)};cueCountdown=null;val t=target?:return@launch;if(cubeState!=t.position.state){awaitingCue=false;message="Cube changed during countdown. Match the setup again.";return@launch};active=ActiveAttempt(t.position.state,"drill",t.stage,settings.cross,settings.session,t.caseId,t.algorithmId,t.position.setup.joinToString(" "),cue=true);attachScramble();awaitingCue=false;elapsed=0.0;message="Go"}
    }
    private fun receiveMove(packet:CubePacket.Move,state:String){
        val now=monotonicMs()
        if(goalSettle.candidate!=null){
            completionJob?.cancel();completionJob=null
            if(goalSettle.continuesWith(packet,now))goalSettle.clear()else finishCompletion()
        }
        visualTurns=(visualTurns+VisualTurn(++visualId,state,packet.move,packet.cubeMs,packet.recovered)).takeLast(128)
        val before=cubeState;cubeState=state;val attempt=active
        if(attempt==null){
            if(target!=null||scrambleLoading){scrambleTracker.observe(before,state,packet);scrambleSpeed=scrambleTracker.stats()}
            target?.let{
                val setup=it.position.setup
                if(before!=null&&scrambleProgress(before,setup).matched)scrambleCheckpoint=before
                if(scrambleProgress(state,setup).matched)scrambleCheckpoint=state
                val plan=scrambleCorrection(before,state,packet.move,setup,recoveryMoves)
                clearGuidance();if(plan!=null)recoveryMoves=plan
            }
            if(awaitingCue&&state!=target?.position?.state){awaitingCue=false;cueJob?.cancel();cueCountdown=null}
            maybeReady();refreshGuidance();return
        }
        gyroViewFrame?.let{(basis,pose)->attempt.captureViewFrame(basis,pose,now)}
        val done=attempt.move(packet,state,now);moves=attempt.moves.toList();elapsed=moves.lastOrNull()?.t?:0.0
        if(done||now-lastDraft>=200){lastDraft=now;storage.write("draft",WireJson.encodeToString(attempt.record(catalog,"interrupted","app-closed")))}
        if(done){
            val candidate=goalSettle.offer(packet,elapsed,now)
            send("state")
            completionJob=scope.launch{
                while(goalSettle.candidate===candidate&&active===attempt){
                    val remaining=120.0-(monotonicMs()-candidate.offeredAt)
                    if(remaining<=0){finishCompletion();break}
                    delay(kotlin.math.ceil(remaining).toLong().coerceAtLeast(1))
                }
            }
        }
    }
    private fun finishCompletion(){
        val candidate=goalSettle.take()?:return
        completionJob=null
        val attempt=active?:return
        val completedTarget=target
        val r=attempt.record(catalog).let{if(candidate.verified)it else it.copy(quality=it.quality+"unverified-final-state")}
        active=null;target=null;scrambleCheckpoint=null;latest=r;elapsed=r.duration
        if(!candidate.verified)pendingVerification=r.id to (candidate.packet.serial and 255)
        storage.write("draft","");save(r);if(!candidate.verified)send("state");message="Saved · ${formatTime(r.duration)}"
        if(r.kind=="solve")prepareNextScramble(false)
        else if(completedTarget!=null){target=completedTarget;scrambleTracker.reset();scrambleSpeed=ScrambleStats();clearGuidance();refreshGuidance()}
        if(r.kind=="solve"||r.stage=="Cross")launch{
            val chosenCross=r.analysis?.cross?.takeIf{it.isNotBlank()}?:r.cross
            val cross=WireJson.decodeFromString<CrossSolution>(request("cross","POST",buildJsonObject{put("state",r.initialState);put("cross",chosenCross)}.toString())).copy(cross=chosenCross)
            val latestVersion=records.firstOrNull{it.id==r.id}?:return@launch
            if((latestVersion.analysis?.cross?:latestVersion.cross)==chosenCross)save(latestVersion.copy(crossOptimal=cross))
        }
    }
    private fun clearGuidance(){
        recoveryRevision++;recoveryJob?.cancel();recoveryJob=null
        recoveryMoves=emptyList();recoveryBusy=false;recoveryError=null
    }
    private fun refreshGuidance(){
        val state=cubeState?:return;val current=target?:return
        if(active!=null){clearGuidance();return}
        if(scrambleProgress(state,current.position.setup).matched){scrambleCheckpoint=state;clearGuidance();return}
        val checkpoint=scrambleCheckpoint?.takeIf{scrambleProgress(it,current.position.setup).matched}?:Cube.solved
        scrambleCheckpoint=checkpoint
        if(recoveryMoves.isNotEmpty()||recoveryJob?.isActive==true||recoveryError!=null)return
        val revision=recoveryRevision;val generation=targetGeneration;val destination=checkpoint
        fun stillCurrent()=revision==recoveryRevision&&generation==targetGeneration&&cubeState==state&&target==current&&scrambleCheckpoint==destination&&active==null
        recoveryBusy=true
        recoveryJob=scope.launch {
            try {
                delay(180)
                val result=WireJson.parseToJsonElement(request("recovery","POST",buildJsonObject{put("state",state);put("target",destination)}.toString())).jsonObject
                if(!stillCurrent())return@launch
                val plan=result.getValue("moves").jsonArray.map{it.jsonPrimitive.content}
                check(Cube.apply(state,plan)==destination){"Invalid correction"}
                recoveryMoves=plan;recoveryError=null
            }catch(e:CancellationException){throw e}
            catch(e:Exception){if(stillCurrent())recoveryError="Correction unavailable offline. Reconnect to retry."}
            finally{if(stillCurrent())recoveryBusy=false}
        }
    }

    private fun fault(reason:String){if(reason in setOf("connection-lost","bluetooth-off","state-timeout","app-closed")){orientation=null;gyroViewFrame=null;orientationAt=null;gyroLive=false};scrambleTracker.reset();scrambleSpeed=ScrambleStats();cancel(reason);cubeState=null;val pending=pendingVerification;pendingVerification=null;if(pending!=null)records.find{it.id==pending.first}?.let{save(it.copy(status="interrupted",penalty="DNF",quality=it.quality+reason))};message="Tracking interrupted: ${reason.replace('-',' ')}. Request the state before another attempt."}
    fun cancel(reason:String="user-stopped"){completionJob?.cancel();completionJob=null;goalSettle.clear();targetGeneration++;scrambleLoading=false;clearGuidance();cueJob?.cancel();cueCountdown=null;awaitingCue=false;active?.takeIf{it.moves.isNotEmpty()}?.let{save(it.record(catalog,"interrupted",reason))};active=null;target=null;scrambleCheckpoint=null;storage.write("draft","")}
    fun updateRecord(record:AttemptRecord){save(record);review=records.firstOrNull{it.id==record.id}?:refreshAnalysis(record,catalog)}
    fun deleteRecord(record:AttemptRecord){launch{
        // Wait for any upload/bootstrap in flight before deleting, so its stale
        // response cannot restore the removed solve or re-upload its pending copy.
        syncMutex.withLock {
            if(settings.token.isNotBlank())request("records/${record.id}","DELETE","{}")
            records=records.filterNot{it.id==record.id};pending.removeAll{it.id==record.id};persistPending()
            storage.write("cache",WireJson.encodeToString(Bootstrap(records=records,preferences=preferences)))
            if(review?.id==record.id)review=null
            if(latest?.id==record.id)latest=null
        }
        sync()
    }}
    fun selectVariant(case:CubeCase,index:Int){selectedVariant=index;val current=preferences[case.id]?:Preferences();preferences=preferences+(case.id to current.copy(variant=index,algorithm=null));storage.write("cache",WireJson.encodeToString(Bootstrap(records=records,preferences=preferences)));if(settings.token.isNotBlank())launch{request("preferences","PUT",WireJson.encodeToString(preferences));sync()}}
    fun toggleFavorite(case:CubeCase){val p=preferences[case.id]?:Preferences();preferences=preferences+(case.id to p.copy(favorite=!p.favorite));storage.write("cache",WireJson.encodeToString(Bootstrap(records=records,preferences=preferences)));if(settings.token.isNotBlank())launch{request("preferences","PUT",WireJson.encodeToString(preferences));sync()}}
    fun manualTime(seconds:Double){require(seconds>0&&seconds<=86400);val r=AttemptRecord(newId(),nowIso(),"solve","manual",duration=seconds*1000,initialState=Cube.solved,session=settings.session,cross=settings.cross,scramble=target?.position?.setup?.joinToString(" ")?:"");latest=r;save(r);prepareNextScramble(false)}
    fun exportBackup():String=WireJson.encodeToString(Bootstrap(records=records,preferences=preferences))
    fun importBackup(raw:String){launch{
        val incoming=withContext(Dispatchers.Default){importNativeRecords(raw).map{refreshAnalysis(it,catalog)}}
        val ids=incoming.map{it.id}.toSet()
        records=(records.filterNot{it.id in ids}+incoming).sortedBy{it.createdAt}
        pending.removeAll{it.id in ids};pending.addAll(incoming);persistPending()
        storage.write("cache",WireJson.encodeToString(Bootstrap(records=records,preferences=preferences)))
        message="Backup imported and saved locally";sync()
    }}
    fun close(){cancel("app-closed");scope.launch{transport.disconnect();transport.close()}.invokeOnCompletion{scope.cancel()}}
}

fun formatTime(ms:Double?):String {
    if(ms==null)return "—";if(!ms.isFinite())return "DNF"
    val centis=kotlin.math.round(ms/10).toLong();val seconds=centis/100;val fraction=(centis%100).toString().padStart(2,'0')
    return if(seconds>=60)"${seconds/60}:${(seconds%60).toString().padStart(2,'0')}.$fraction" else "$seconds.$fraction"
}
