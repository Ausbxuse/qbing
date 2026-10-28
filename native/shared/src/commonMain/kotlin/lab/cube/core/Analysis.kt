package lab.cube.core

private data class DetectedCross(val face:Char,val source:String,val candidates:List<String>,val evidence:String)
private fun configuredCross(record:AttemptRecord):Char=record.cross.singleOrNull()?.takeIf{it in Cube.faces}?:'U'

private fun detectCross(record:AttemptRecord,states:List<String>):DetectedCross {
    val configured=configuredCross(record)
    if(record.kind=="drill"||record.stage.isNotBlank()&&record.stage!="solve")
        return DetectedCross(configured,"configured",listOf(configured.toString()),"configured")
    val crossAt=IntArray(6){Int.MAX_VALUE};val f2lAt=IntArray(6){Int.MAX_VALUE}
    for((index,state)in states.withIndex())for((f,face)in Cube.faces.withIndex()){
        val progress=Cube.progress(state,face)
        if(progress.cross&&crossAt[f]==Int.MAX_VALUE)crossAt[f]=index
        if(progress.f2l&&f2lAt[f]==Int.MAX_VALUE)f2lAt[f]=index
    }
    val hasF2l=f2lAt.any{it!=Int.MAX_VALUE}
    val evidence=if(hasF2l)f2lAt else crossAt
    val first=evidence.minOrNull()?:Int.MAX_VALUE
    if(first==Int.MAX_VALUE)return DetectedCross(configured,"configured",listOf(configured.toString()),"configured")
    val candidates=Cube.faces.filterIndexed{index,_->evidence[index]==first}
    val chosen=if(configured in candidates)configured else candidates.first()
    return DetectedCross(chosen,if(candidates.length==1)"detected"else "ambiguous",candidates.map{it.toString()},if(hasF2l)"f2l"else "cross")
}

/** Upgrade derived data only; the original chosen setting and sensor events remain unchanged. */
fun refreshAnalysis(record:AttemptRecord,catalog:Catalog):AttemptRecord {
    if(record.source=="manual"||record.moves.isEmpty())return record.copy(analysis=null,crossOptimal=null)
    val current=record.analysis
    val analysis=if(current!=null&&current.version>=4&&current.cross.length==1&&current.cross[0] in Cube.faces)current
        else runCatching{analyze(record,catalog)}.getOrNull()
    val selected=analysis?.cross?.singleOrNull()
    val optimal=record.crossOptimal?.takeIf{solution->
        val originalFace=solution.cross?.singleOrNull()
            ?:current?.cross?.singleOrNull()?.takeIf{it in Cube.faces}?:configuredCross(record)
        selected!=null&&originalFace==selected&&runCatching{
            Cube.progress(Cube.apply(record.initialState,solution.moves),selected).cross
        }.getOrDefault(false)
    }?.copy(cross=analysis?.cross)
    return record.copy(analysis=analysis,crossOptimal=optimal,scrambleStats=if(record.scrambleMoves.isNotEmpty())scrambleStats(record.scrambleMoves,record.scrambleQuality)else record.scrambleStats)
}

fun analyze(record: AttemptRecord,catalog: Catalog): Analysis {
    val states=buildList{var state=record.initialState;add(state);for(move in record.moves){state=Cube.turn(state,move.move);add(state)}}
    val chosen=detectCross(record,states)
    val stages=listOf("Cross","F2L","OLL","PLL");val cross=chosen.face;var state=record.initialState;var prev=Cube.progress(state,cross)
    val milestone=mutableMapOf<String,Double?>("Cross" to if(prev.cross)0.0 else null,"F2L" to if(prev.f2l)0.0 else null,"OLL" to if(prev.oll)0.0 else null,"PLL" to if(prev.solved)0.0 else null)
    val cases=mutableMapOf("OLL" to if(prev.f2l)Cube.recognize(state,"OLL",cross,catalog) else null,"PLL" to if(prev.oll)Cube.recognize(state,"PLL",cross,catalog) else null)
    val pairTimes=prev.pairs.filterValues{it}.keys.toMutableSet();val pairEvents=mutableListOf<PairEvent>();val pauses=mutableListOf<Pause>();val signals=mutableListOf<ReviewSignal>();val moveStages=mutableListOf<String>();var previous=0.0
    for((i,m) in record.moves.withIndex()) {
        val stage=stages.firstOrNull{milestone[it]==null}?:"PLL";moveStages.add(stage)
        val gap=m.t-previous
        if(i>0&&gap>=500)pauses.add(Pause(i,previous,gap,stage,m.recovered||record.moves[i-1].recovered))
        if(i>0&&Cube.inverse(listOf(m.move))[0]==record.moves[i-1].move)signals.add(ReviewSignal("undo",i,stage,"${record.moves[i-1].move} ${m.move}",m.t))
        state=states[i+1];val p=Cube.progress(state,cross)
        for((slot,done) in p.pairs)if(done&&prev.pairs[slot]!=true&&p.cross&&pairTimes.add(slot))pairEvents.add(PairEvent(slot,m.t,i))
        if(milestone["Cross"]==null&&p.cross)milestone["Cross"]=m.t
        if(milestone["Cross"]!=null&&milestone["F2L"]==null&&p.f2l){milestone["F2L"]=m.t;cases["OLL"]=Cube.recognize(state,"OLL",cross,catalog)}
        if(milestone["F2L"]!=null&&milestone["OLL"]==null&&p.oll){milestone["OLL"]=m.t;cases["PLL"]=Cube.recognize(state,"PLL",cross,catalog)}
        if(milestone["OLL"]!=null&&milestone["PLL"]==null&&p.solved)milestone["PLL"]=m.t
        prev=p;previous=m.t
    }
    val weights=moveWeights(record.moves.map{it.move})
    var start=0.0
    val splits=stages.map{stage->val end=milestone[stage];val split=Split(stage,start,end,end?.let{it-start},moveStages.indices.filter{moveStages[it]==stage}.sumOf{weights[it]},pauses.filter{it.stage==stage}.sumOf{it.duration},cases[stage],rawMoves=moveStages.count{it==stage});if(end!=null)start=end;split}
    val paused=pauses.sumOf{it.duration}
    return Analysis(splits,pauses,signals,pairEvents,weights.sum(),record.duration,paused,if(record.duration>0)paused/record.duration else 0.0,if(record.duration>0)weights.sum()/(record.duration/1000) else null,state,
        version=4,cross=cross.toString(),crossSource=chosen.source,crossCandidates=chosen.candidates,crossEvidence=chosen.evidence)
}
fun adjusted(r:AttemptRecord):Double=if(r.penalty=="DNF"||r.status!="complete")Double.POSITIVE_INFINITY else r.duration+if(r.penalty=="+2")2000 else 0
fun average(records:List<AttemptRecord>,size:Int):Double? {
    if(records.size<size)return null
    val times=records.takeLast(size).map(::adjusted).sorted();val trim=if(size>=5)kotlin.math.ceil(size*.05).toInt() else 0
    return times.subList(trim,times.size-trim).average()
}
fun median(values:List<Double>):Double? {val a=values.filter{it.isFinite()}.sorted();return if(a.isEmpty())null else (a[(a.size-1)/2]+a[a.size/2])/2}

class ActiveAttempt(val initialState:String,val kind:String,val stage:String,val cross:String,val session:String,val caseId:String?=null,val algorithmId:String?=null,val scramble:String="",val cue:Boolean=false,val readyAt:Double=monotonicMs()) {
    private val id = newId()
    private val createdAt = nowIso()
    var scrambleMoves:List<MoveSample> = emptyList()
    var scrambleQuality:List<String> = emptyList()
    var startedAt:Double?=null;private set
    private var firstCube:Double?=null
    private val gyroRecorder=GyroRotationRecorder()
    private val rotations=mutableListOf<RotationSample>()
    private fun savePose(notation:String,pose:List<Float>,now:Double){
        val latest=moves.lastOrNull();val received=latest?.raw?.receivedAt
        val t=if(latest!=null&&received!=null)latest.t+(now-received).coerceAtLeast(0.0) else now-(startedAt?:now)
        rotations.add(RotationSample(notation,t,pose))
    }
    fun captureViewFrame(basis:List<Float>,pose:List<Float>,now:Double=monotonicMs()){
        if(rotations.isNotEmpty()&&!gyroRecorder.liveFrame)return
        gyroRecorder.setViewFrame(basis,pose,startedAt!=null)?.let{(notation,p)->savePose(notation,p,now)}
    }
    fun orientation(q:List<Float>,now:Double=monotonicMs()){
        gyroRecorder.observe(q,startedAt!=null)?.let{(notation,pose)->savePose(notation,pose,now)}
    }
    val moves=mutableListOf<MoveSample>();val quality=mutableSetOf<String>()
    fun move(packet:CubePacket.Move,state:String,now:Double=monotonicMs()):Boolean {
        if(startedAt==null){startedAt=now;firstCube=packet.cubeMs}
        if(firstCube==null&&!packet.recovered)firstCube=estimatedOrigin(moves,packet.cubeMs)
        val regressed=!packet.recovered&&deviceClockRegressed(moves,packet.cubeMs)
        if(regressed)quality.add("clock-reset")
        val deviceElapsed=when {
            packet.recovered&&packet.cubeMs==null->estimatedRecoveredElapsed(moves)
            packet.cubeMs!=null&&firstCube!=null->packet.cubeMs-firstCube!!
            else->now-startedAt!!
        }
        if(deviceElapsed<0)quality.add("clock-reset")
        if(!packet.recovered)reflowRecoveredTimes(moves,firstCube,packet.cubeMs,"clock-reset" in quality)
        val t=maxOf(moves.lastOrNull()?.t?:0.0,if(deviceElapsed<0||regressed)now-startedAt!! else deviceElapsed)
        if(packet.recovered)quality.add("estimated-timing")
        moves.add(MoveSample(packet.move,t,packet.recovered,RawMove(packet.serial,packet.localMs,packet.cubeMs,packet.localMs,now)))
        return Cube.goal(state,stage,cross.first())
    }
    fun record(catalog:Catalog,status:String="complete",reason:String?=null):AttemptRecord {
        val inspection=startedAt?.minus(readyAt)
        val r=AttemptRecord(id,createdAt,kind,status=status,penalty=if(status=="complete")"" else "DNF",duration=moves.lastOrNull()?.t?:0.0,initialState=initialState,cross=cross,session=session,stage=stage,caseId=caseId,scramble=scramble,moves=moves.toList(),quality=quality.toList()+listOfNotNull(reason),inspection=inspection,recognitionMs=if(cue)inspection else null,recognitionSource=if(cue)"cue" else null,selectedAlgorithmId=algorithmId)
        return r.copy(rotationFrame=if(gyroRecorder.initialPose!=null)"live-v1"else null,initialOrientation=gyroRecorder.initialPose?.values(),rotations=rotations.filter{it.t<=r.duration},analysis=analyze(r,catalog),scrambleMoves=scrambleMoves,scrambleQuality=scrambleQuality,scrambleStats=scrambleStats(scrambleMoves,scrambleQuality))
    }
}
