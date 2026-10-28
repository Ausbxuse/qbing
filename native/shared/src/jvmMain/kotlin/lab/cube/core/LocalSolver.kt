package lab.cube.core

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.*
import lab.cube.resources.Res
import org.jetbrains.compose.resources.ExperimentalResourceApi
import org.mozilla.javascript.Context
import org.mozilla.javascript.ScriptableObject

// Serial background interpreter; no WebView, network, DOM, or UI work.
private val solverDispatcher=Dispatchers.Default.limitedParallelism(1)
private var solverScope:ScriptableObject?=null
@OptIn(ExperimentalResourceApi::class)
actual suspend fun localSolver(route:String,body:String?):String=withContext(solverDispatcher) {
    if(route=="scramble"){
        cs.min2phase.Search.init()
        val state=cs.min2phase.Tools.randomCube()
        val scramble=cs.min2phase.Search().solution(state,21,100000000,50,cs.min2phase.Search.INVERSE_SOLUTION).trim().split(Regex("\\s+")).joinToString(" ")
        check(!scramble.startsWith("Error")&&Cube.apply(Cube.solved,scramble.split(" "))==state){"Scramble verification failed"}
        return@withContext buildJsonObject{put("scramble",scramble);put("state",state)}.toString()
    }
    val source=if(solverScope==null)Res.readBytes("files/solver.js").decodeToString() else null
    val context=Context.enter()
    try {
        context.optimizationLevel=-1
        context.languageVersion=Context.VERSION_ES6
        val scope=solverScope?:context.initSafeStandardObjects().also{context.evaluateString(it,source!!,"cstimer-solvers",1,null);solverScope=it}
        val request=buildJsonObject{put("route",route);put("body",body?.let{WireJson.parseToJsonElement(it)}?:JsonNull)}.toString()
        val function=scope.get("nativeRequest",scope) as org.mozilla.javascript.Function
        Context.toString(function.call(context,scope,scope,arrayOf(request)))
    }finally{Context.exit()}
}
