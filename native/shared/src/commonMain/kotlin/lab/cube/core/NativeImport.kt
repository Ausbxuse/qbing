package lab.cube.core

import kotlinx.serialization.json.*

/** Explicit JSON import only. Browser profiles are never read. */
fun importNativeRecords(raw:String):List<AttemptRecord>{
 val root=WireJson.parseToJsonElement(raw).jsonObject
 root["records"]?.let{return WireJson.decodeFromJsonElement<List<AttemptRecord>>(it).onEach{r->require(r.id.isNotBlank()&&r.duration.isFinite()&&r.duration>=0){"Invalid native record"}}}
 val records=mutableListOf<AttemptRecord>()
 for((key,value)in root){
  if(!Regex("session[0-9]+").matches(key))continue
  val rows=(if(value is JsonPrimitive&&value.isString)WireJson.parseToJsonElement(value.content)else value).jsonArray
  for((index,entry)in rows.withIndex()){
   val row=entry.jsonArray;val time=row[0].jsonArray;val duration=time[1].jsonPrimitive.double
   require(duration.isFinite()&&duration>=0){"Invalid recorded time"}
   val scramble=row[1].jsonPrimitive.content
   val initial=Cube.apply(Cube.solved,scramble.trim().split(Regex("\\s+")).filter{it.isNotBlank()})
   val extra=row.getOrNull(4) as? JsonArray
   val moves=extra?.getOrNull(0)?.jsonPrimitive?.content.orEmpty().trim().split(Regex("\\s+")).filter{it.isNotBlank()}.map{token->
    val split=token.lastIndexOf('@');require(split>0){"Invalid timed turn"};val move=token.substring(0,split);require(Regex("[URFDLB](2|')?").matches(move)){"Unsupported turn notation"};val t=token.substring(split+1).toDouble();require(t.isFinite()&&t>=0);MoveSample(move,t)
   }
   val metadata=(row.getOrNull(5) as? JsonObject)?.get("localReview")
   records+=AttemptRecord(id="cstimer-$key-$index-${row.getOrNull(3)?.jsonPrimitive?.longOrNull?:0}",createdAt=isoFromEpoch(row.getOrNull(3)?.jsonPrimitive?.longOrNull?:0),kind="solve",source=if(moves.isEmpty())"manual"else "bluetooth",penalty=when(time[0].jsonPrimitive.int){-1->"DNF";2000->"+2";else->""},duration=duration,initialState=initial,session="Imported $key",scramble=scramble,moves=moves,notes=row.getOrNull(2)?.jsonPrimitive?.content.orEmpty(),localReview=metadata?.let{WireJson.decodeFromJsonElement<SolveReview>(it)}?:SolveReview(),importedSource=entry)
  }
 }
 require(records.isNotEmpty()){"No supported 3×3 sessions found in this JSON backup"}
 return records
}
expect fun isoFromEpoch(seconds:Long):String
