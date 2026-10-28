package lab.cube.ui

import lab.cube.core.ScrambleProgress

/** A recovery move owns the existing cursor until the checkpoint is restored. */
internal fun scrambleDisplayTokens(setup:List<String>,progress:ScrambleProgress?,repairs:List<String>):List<Pair<String,Int>> = buildList {
    for(i in 0..setup.size){
        if(i==(progress?.completed?:0))repairs.forEachIndexed{j,move->add(move to if(j==0)2 else 3)}
        val move=setup.getOrNull(i)?:continue
        val kind=when {progress!=null&&i==progress.completed&&progress.partialMoves.contains(i)->4;progress?.done?.contains(i)==true->1;progress!=null&&i==progress.completed&&progress.nextMoves.contains(i)->2;else->0}
        add(move to if(repairs.isNotEmpty()&&kind in listOf(2,4))0 else kind)
    }
}
