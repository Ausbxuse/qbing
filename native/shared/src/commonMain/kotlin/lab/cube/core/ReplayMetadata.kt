package lab.cube.core

/** Preserve recorded local poses if an older server omits optional replay metadata. */
internal fun retainReplayMetadata(remote:AttemptRecord,local:AttemptRecord?):AttemptRecord {
 if(local==null||remote.id!=local.id||remote.initialState!=local.initialState||remote.duration!=local.duration||remote.moves.size!=local.moves.size||remote.moves.zip(local.moves).any{(a,b)->a.move!=b.move||a.t!=b.t})return remote
 if(remote.rotationFrame?.let{it!="live-v1"}==true)return remote
 initialLivePose(remote)?.let{start->
  return if(remote.rotations.isEmpty()&&initialLivePose(local)?.let(start::closeTo)==true)remote.copy(rotations=local.rotations)else remote
 }
 if(remote.rotations.isNotEmpty()&&remote.rotations!=local.rotations)return remote
 return remote.copy(rotations=local.rotations,rotationFrame=local.rotationFrame,initialOrientation=local.initialOrientation)
}
