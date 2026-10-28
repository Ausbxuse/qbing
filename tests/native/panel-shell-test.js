import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import St from 'gi://St';
import Pango from 'gi://Pango';
import Clutter from 'gi://Clutter';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
export default class Test extends Extension {
 enable(){this.id=GLib.timeout_add(GLib.PRIORITY_DEFAULT,500,()=>{
  if(GLib.getenv('FACET_BACKGROUND_TEST')==='1'){
   global.context.unsafe_mode=true; // Private headless bus/display only.
   this.keyboard??=Clutter.get_default_backend().get_default_seat().create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
   const windows=global.get_window_actors().map(actor=>actor.meta_window);
   const other=windows.find(win=>win.get_title()==='Other application');
   const target=other??windows.find(win=>win.get_title()==='Facet');
   if(target && (other || Main.extensionManager.lookup('cstimer-companion@local')?.stateObj?._state?.windowVisible)){Main.overview.hide();Main.activateWindow(target);}
   const file=GLib.getenv('FACET_FOCUS_REPORT');
   if(file)Gio.File.new_for_path(file).replace_contents(JSON.stringify({active:!!other&&global.display.focus_window===other,windows:windows.map(win=>win.get_title()),focus:global.display.focus_window?.get_title(),overview:Main.overview.visible}),null,false,Gio.FileCreateFlags.REPLACE_DESTINATION,null);
   return GLib.SOURCE_CONTINUE;
  }
  if(GLib.getenv('CSTIMER_PANEL_TEST')!=='1')return GLib.SOURCE_REMOVE;
  const e=Main.extensionManager.lookup('cstimer-companion@local');
  const ui=e?.stateObj;
  if(!ui?._state||(!ui._state.connected&&!this.scenarios)||Main.layoutManager._startingUp)return GLib.SOURCE_CONTINUE;
  Main.overview.hide();
  if(!this.longTitle){this.longTitle=new St.Label({text:'A very long focused editor window title '.repeat(40),x_expand:true});this.longTitle.clutter_text.ellipsize=Pango.EllipsizeMode.END;Main.panel._leftBox.insert_child_at_index(this.longTitle,0);ui._fit();return GLib.SOURCE_CONTINUE;}
  const geometry=()=>[ui._button.width,ui._scrambleBox.get_transformed_position()[0],ui._tps.get_transformed_position()[0],ui._moves.get_transformed_position()[0],...ui._scrambleRows.flatMap(label=>[...label.get_first_child().clutter_text.get_transformed_position(),label.get_first_child().clutter_text.get_layout().get_baseline()/Pango.SCALE])];
  if(!this.metricBefore){
   this.metricBefore=geometry();this.savedState=ui._state;this.savedPoll=ui._poll;
   ui._poll=()=>{};ui._state={...ui._state,time:'1:59.99',tpsLabel:'12.34',moves:126};ui._render();
   return GLib.SOURCE_CONTINUE;
  }
  if(!this.metricAfter){
   this.metricAfter=geometry();this.metricsStable=JSON.stringify(this.metricBefore)===JSON.stringify(this.metricAfter);
   ui._poll=this.savedPoll;ui._state=this.savedState;ui._render();return GLib.SOURCE_CONTINUE;
  }
  if(!this.rowChecks){
   this.rowChecks=[];this.rowBase=geometry();this.savedState=ui._state;this.savedPoll=ui._poll;ui._poll=()=>{};
   const tokens=this.savedState.tokens;
   const cursor=(index,kind)=>tokens.map((t,i)=>({...t,kind:i<index?1:i===index?kind:0}));
   const complete=tokens.map(t=>({...t,kind:1}));
   this.scenarios=[{tokens:cursor(3,4)},{tokens:cursor(15,2)},{tokens:complete},{tokens:complete,running:true},{tokens:cursor(3,2)},{tokens:complete,connected:false}];
   this.scenarioIndex=0;ui._state={...this.savedState,...this.scenarios[0]};ui._render();return GLib.SOURCE_CONTINUE;
  }
  if(this.scenarioIndex<this.scenarios.length){
   const scenario=this.scenarios[this.scenarioIndex];
   this.rowChecks.push({sameGeometry:JSON.stringify(geometry())===JSON.stringify(this.rowBase),ready:ui._readyIcon.visible,expected:this.scenarioIndex===2});
   this.scenarioIndex++;
   ui._state=this.scenarioIndex<this.scenarios.length?{...this.savedState,...this.scenarios[this.scenarioIndex]}:this.savedState;
   if(this.scenarioIndex===this.scenarios.length)ui._poll=this.savedPoll;
   ui._render();return GLib.SOURCE_CONTINUE;
  }
  const data={metricGap:ui._tps.get_transformed_position()[0]-ui._scrambleBox.get_transformed_position()[0]-ui._scrambleBox.width,buttonLeft:ui._button.get_transformed_position()[0],notationRight:ui._scrambleBox.get_transformed_position()[0]+ui._scrambleBox.width,metricsLeft:ui._tps.get_transformed_position()[0],trailingPadding:ui._button.get_transformed_position()[0]+ui._button.width-ui._moves.get_transformed_position()[0]-ui._moves.width,rowChecks:this.rowChecks,rowGeometry:geometry(),metricsStable:this.metricsStable,state:e.state,errors:e.error??'',time:ui._time.text,stats:`${ui._tps.text} · ${ui._moves.text}`,rows:ui._scrambleRows.map(row=>({text:row.get_children().map(label=>label.text).join(' '),lines:Math.max(...row.get_children().map(label=>label.clutter_text.get_layout().get_line_count()))})),width:ui._button.width,panelWidth:Main.panel.width,centerWidth:Main.panel._centerBox.width,scrambleVisible:ui._scrambleBox.visible,buttonRight:ui._button.get_transformed_position()[0]+ui._button.width,clockLeft:Main.panel._centerBox.get_transformed_position()[0],panelOpacity:Main.panel.opacity,buttonOpacity:ui._button.get_paint_opacity(),overview:Main.overview.visible};
  Gio.File.new_for_path(GLib.getenv('CSTIMER_PANEL_REPORT')).replace_contents(JSON.stringify(data),null,false,Gio.FileCreateFlags.REPLACE_DESTINATION,null);
  // This is a private headless bus/display, never the real desktop.
  global.context.unsafe_mode=true;
  this.id=0;return GLib.SOURCE_REMOVE;
 });}
 key(keyval, pressed){
  this.keyboard.notify_keyval(GLib.get_monotonic_time(),keyval,pressed?Clutter.KeyState.PRESSED:Clutter.KeyState.RELEASED);
 }
 disable(){this.longTitle?.destroy();if(this.id){try{GLib.source_remove(this.id);}catch(_){}this.id=0;}}
}
