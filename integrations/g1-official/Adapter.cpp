// External controls/display adapter. No synth engine or modified plugin.
// Reuses Animatek NME's GPL-3.0 protocol and patch-model implementation.
#include <juce_gui_basics/juce_gui_basics.h>
#include "KeyboardNmeData.h"
#include "midi/ConnectionManager.h"
#include "midi/DirectLink.h"
#include "model/PatchParser.h"
#include "model/PchFileIO.h"
#include "format/ValueFormatters.h"
#include <atomic>
#include <csignal>
#include <iostream>
#include "MidiActivity.h"
#include "ParameterNames.h"
#include "SequencerMeters.h"
#include "model/ThemeData.h"
#include "model/LightMeterLayout.h"
#include <sys/socket.h>
#include <cerrno>
#include <map>
#include <set>
#include <deque>
#include <tuple>
#if JUCE_MAC
#include <CoreFoundation/CoreFoundation.h>
#endif
using juce::var;
static std::atomic<bool> stopping{false};
static void stop(int){stopping=true;}
static var object(){return new juce::DynamicObject();}
static void put(var& o,const juce::Identifier& key,var value){o.getDynamicObject()->setProperty(key,std::move(value));}
static int integer(const var& r,const char* key,int lo,int hi){auto v=r[key];if(!v.isInt() || int(v)<lo || int(v)>hi)throw juce::String("Invalid ")+key;return int(v);}
static bool pipelineTrace(){
    if(!std::getenv("MK3_TRACE_PIPELINE"))return false;
    const char* until=std::getenv("MK3_TRACE_UNTIL_MS");
    return !until || juce::Time::currentTimeMillis()<std::atoll(until);
}
class Adapter final : public NmProtocol::Listener {
public:
    using ParameterKey=std::tuple<int,int,int>;
    struct OriginalValue {int value,min,max,type;};
    using OriginalValues=std::map<ParameterKey,OriginalValue>;
    OriginalValues parameterValues(int slot) const {
        OriginalValues values;if(!patches[slot])return values;
        for(int section:{1,0})for(const auto& module:patches[slot]->getContainer(section).getModules())for(const auto& p:module->getParameters()){
            auto* d=p.getDescriptor();if(d->paramClass=="parameter" && d->minValue>=0 && d->maxValue<=127)
                values[{section,module->getContainerIndex(),d->index}]={p.getValue(),d->minValue,d->maxValue,module->getDescriptor()->index};
        }
        return values;
    }
    bool canRestore(int slot) const {return validSnapshot[slot]&&!originals[slot].empty()&&originalGeneration[slot]==generation[slot];}
    bool hasOriginalChanges(int slot) const {
        if(!canRestore(slot)||!patches[slot])return false;
        bool changed=!unconfirmedWrites[slot].empty();size_t count=0;
        for(int section:{1,0})for(const auto& module:patches[slot]->getContainer(section).getModules())for(const auto& p:module->getParameters()){
            const auto* d=p.getDescriptor();if(d->paramClass!="parameter"||d->minValue<0||d->maxValue>127)continue;
            auto original=originals[slot].find({section,module->getContainerIndex(),d->index});
            if(original==originals[slot].end()||original->second.type!=module->getDescriptor()->index||original->second.min!=d->minValue||original->second.max!=d->maxValue)return false;
            ++count;changed=changed||p.getValue()!=original->second.value;
        }
        return changed&&count==originals[slot].size();
    }
    void stopRestore(const juce::String& reason){std::fprintf(stderr,"RESTORE_END slot=%d serial=%llu result=%s\n",restoreSlot,(unsigned long long)restoreSerial,reason.toRawUTF8());restoreBusy=false;restoreQueue.clear();restoreSections.clear();restoreMessage=reason;}
    var restoreState() const {
        auto o=object();put(o,"baselineAvailable",canRestore(active));put(o,"available",hasOriginalChanges(active));put(o,"busy",restoreBusy);put(o,"serial",juce::int64(restoreSerial));
        put(o,"written",restoreWritten);put(o,"total",restoreTotal);put(o,"remaining",int(restoreQueue.size()));put(o,"slot",restoreSlot);put(o,"message",restoreMessage);return o;
    }
    void beginRestore(const var& request){
        const int slot=integer(request,"slot",0,3);
        if(slot!=active||request["patchBinding"].toString()!=patchBinding())throw juce::String("Stale slot restore request");
        if(restoreBusy)throw juce::String("Slot restore already running");
        if(functionHeld)throw juce::String("Release SHIFT before restoring slot values");
        if(!canRestore(slot)||uploadBusy||bankBusy||deferredPatch||pendingSelectedSlot>=0||connection.isFetchingPatch()||connection.isUploadingPatch())throw juce::String("Wait for a confirmed slot before restoring");
        const auto current=parameterValues(slot);const auto& baseline=originals[slot];
        if(current.size()!=baseline.size())throw juce::String("Patch structure changed; original values cannot be restored safely");
        for(const auto& [key,value]:baseline){auto i=current.find(key);if(i==current.end()||i->second.type!=value.type||i->second.min!=value.min||i->second.max!=value.max)throw juce::String("Patch structure changed; original values cannot be restored safely");}
        targets.clear();parameterTargets.clear();mixerTargets.clear();dirtySections.clear();readAt=0;
        restoreSlot=slot;restoreGeneration=generation[slot];restoreBusy=true;++restoreSerial;restoreWritten=0;
        for(const auto& [key,value]:baseline)if(current.at(key).value!=value.value||unconfirmedWrites[slot].count(key)){restoreQueue.push_back(key);restoreSections.insert(std::get<0>(key));}
        restoreTotal=int(restoreQueue.size());std::fprintf(stderr,"RESTORE_BEGIN slot=%d serial=%llu changed=%d params=%zu\n",slot,(unsigned long long)restoreSerial,restoreTotal,baseline.size());restoreMessage="Restoring original slot values…";restoreDeadline=juce::Time::getMillisecondCounterHiRes()+10000+restoreTotal*4;
        if(restoreQueue.empty()){stopRestore("Original slot values restored");return;}
        if(readInFlight)readDeadline=std::min(readDeadline,juce::Time::getMillisecondCounterHiRes()+150.0);
    }
    void advanceRestore(double now){
        if(!restoreBusy)return;
        if(now>=restoreDeadline){stopRestore("Slot restore timed out; press CLEAR to retry");return;}
        if(!client||!connection.isConnected()||restoreGeneration!=generation[restoreSlot]||!validSnapshot[restoreSlot]){stopRestore("Slot restore interrupted");return;}
        if(restoreQueue.empty()||now<nextRestoreWrite||readInFlight||slotStage||connection.isFetchingPatch()||connection.isUploadingPatch()||connection.getProtocol().isWaitingForReply())return;
        for(int count=0;count<4&&!restoreQueue.empty();++count){const auto key=restoreQueue.front();restoreQueue.pop_front();const auto [section,module,param]=key;
            sendBoundParameter(restoreSlot,section,module,param,originals[restoreSlot].at(key).value);++restoreWritten;
        }
        nextRestoreWrite=now+16;
        if(restoreQueue.empty()){for(int section:restoreSections)dirtySections.insert({restoreSlot,section});readAt=now+16;}
    }
    void confirmRestore(int slot,int section){
        if(!restoreBusy||slot!=restoreSlot||!restoreQueue.empty()||!restoreSections.count(section))return;
        const auto actual=parameterValues(slot);
        for(const auto& [key,value]:originals[slot])if(std::get<0>(key)==section){auto i=actual.find(key);if(i==actual.end()||i->second.value!=value.value){stopRestore("Original slot values were not confirmed; press CLEAR to retry");return;}}
        restoreSections.erase(section);if(restoreSections.empty())stopRestore("Original slot values restored");
    }
    Adapter(int directPort,int jsonPort):directPort(directPort),jsonPort(jsonPort),parser(descriptions) {
        descriptions.loadFromXmlString(juce::String::fromUTF8(KeyboardNmeData::modules_xml,KeyboardNmeData::modules_xmlSize));
        if(descriptions.getModuleCount()==0)throw juce::String("Missing module descriptions");
        if(!meterTheme.loadFromXmlString(juce::String::fromUTF8(KeyboardNmeData::classictheme_xml,KeyboardNmeData::classictheme_xmlSize)))throw juce::String("Missing module meter layout");
        connection.getProtocol().addListener(this);
        connection.setSynthSettingsCallback([this](const SynthSettings& s){settings=s;settingsKnown=true;if(s.hasExtended){active=s.activeSlot;slotKnown=true;if(active==pendingSelectedSlot)pendingSelectedSlot=-1;for(int i=0;i<4;++i){voices[i]=s.slotVoiceCount[i];if(i!=active && !functionHeld)pinned[i]=s.slotSelected[i]!=0;}}});
        connection.setPatchLoadIncompleteCallback([this](int slot,int,int){incomplete[slot]=true;validSnapshot[slot]=false;problem="Incomplete official patch download; previous snapshot retained";if(slot==uploadSlot){uploadBusy=false;uploadOk=false;bankBusy=false;bankOk=false;}});
        connection.setPatchDataCallback([this](const auto& sections,int slot){if(incomplete[slot]){incomplete[slot]=false;return;}auto patch=parser.parse(sections);if(!patch)return;validSnapshot[slot]=true;confirmedPid[slot]=connection.getPatchId(slot);for(auto i=readFailures.begin();i!=readFailures.end();)if(i->first.first==slot)i=readFailures.erase(i);else ++i;patches[slot]=std::move(patch);meterLayouts[slot]=LightMeterLayout::build(patches[slot].get(),&meterTheme);snapshots[slot]=sections;juce::String fingerprint=juce::String(connection.getPatchId(slot))+":"+patches[slot]->getName();for(const auto& k:patches[slot]->knobAssignments)fingerprint+=":"+juce::String(int(k.assigned))+":"+juce::String(k.section)+":"+juce::String(k.module)+":"+juce::String(k.param);if(fingerprints[slot]!=fingerprint){fingerprints[slot]=fingerprint;++generation[slot];}if(originals[slot].empty()||originalPid[slot]!=confirmedPid[slot]){originals[slot]=parameterValues(slot);unconfirmedWrites[slot].clear();originalPid[slot]=confirmedPid[slot];std::fprintf(stderr,"RESTORE_BASELINE slot=%d pid=%d params=%zu\n",slot,confirmedPid[slot],originals[slot].size());}originalGeneration[slot]=generation[slot];if(uploadBusy && slot==uploadSlot){uploadBusy=false;uploadOk=true;}if(bankBusy && slot==uploadSlot){bankBusy=false;bankOk=true;} });
        connection.setUploadCompleteCallback([this]{connection.requestPatch(uploadSlot);});
        connection.setUploadFailedCallback([this](int, const juce::String& reason){uploadBusy=false;uploadOk=false;problem=reason;});
        connection.setSynthErrorCallback([this](int code){problem="Synth error "+juce::String(code);});
        connection.setPatchListCallback([this](const auto&){listingBanks=false;});
        connection.setVoiceCountCallback([this](const int* v){for(int i=0;i<4;++i)voices[i]=v[i];});
        connection.setStatusCallback([this](const auto& status){if(status.state==ConnectionManager::State::Connected){nextSettings=juce::Time::getMillisecondCounterHiRes();} });
        if(!server.createListener(jsonPort,"127.0.0.1"))throw juce::String("Adapter port unavailable");
        bool connected=false;
        for(int attempt=0;attempt<15 && !stopping;++attempt){
            if(connection.connect(DirectLink::deviceId(directPort),DirectLink::deviceId(directPort))){connected=true;break;}
            // Discovery closes its probe first; let the official plugin observe
            // that close before the real single-editor connection arrives.
            juce::Thread::sleep(80);
        }
        if(!connected)throw juce::String("Official DirectLink unavailable or already in use");
    }
    ~Adapter() override {connection.getProtocol().removeListener(this);connection.disconnect();}
    void onNMInfoReceived(const NMInfoMessage& m) override {
        if(m.sc==0x3a&&m.meterStartIndex>=0&&slotKnown&&pendingSelectedSlot<0&&slotStage==0&&validSnapshot[active]&&!uploadBusy&&!bankBusy&&!connection.isFetchingPatch()){
            const int count=std::min(5,int(m.data.size()>0?(m.data.size()-1)/2:0));
            if(count>0){if(!meterStreamSeen){std::fprintf(stderr,"LIVE_METER_STREAM slot=%d pid=%d start=%d pairs=%d\n",active,m.pid,m.meterStartIndex,count);meterStreamSeen=true;}
                sequencerMeters.ingest(active,generation[active],m.meterStartIndex,m.meterValuesB,m.meterValuesA,count,int64_t(juce::Time::getMillisecondCounterHiRes()));if(std::getenv("G1_KEYBOARD_TRACE")&&meterDebugFrames++<8)std::fprintf(stderr,"METER_SAMPLE start=%d b=%d,%d,%d,%d,%d a=%d,%d,%d,%d,%d\n",m.meterStartIndex,m.meterValuesB[0],m.meterValuesB[1],m.meterValuesB[2],m.meterValuesB[3],m.meterValuesB[4],m.meterValuesA[0],m.meterValuesA[1],m.meterValuesA[2],m.meterValuesA[3],m.meterValuesA[4]);}
        }
        if(m.sc==9 && !m.data.empty()){if(active!=(m.data[0]&3))targets.clear();parameterTargets.clear();mixerTargets.clear();active=m.data[0]&3;slotKnown=true;if(active==pendingSelectedSlot)pendingSelectedSlot=-1;for(int i=0;i<4;++i)if(i!=active && !functionHeld)pinned[i]=connection.isSlotEnabled(i);}
        if(m.sc==0x38 && m.newPatchSlot>=0){const int slot=m.newPatchSlot&3;originals[slot].clear();unconfirmedWrites[slot].clear();if(slot==active){sequencerMeters.reset();meterStreamSeen=false;}if(restoreBusy&&restoreSlot==slot)stopRestore("Slot replaced during restore");if(slot==active)targets.clear();parameterTargets.clear();mixerTargets.clear();if(m.newPatchPid>=0)confirmedPid[slot]=m.newPatchPid;patches[slot].reset();validSnapshot[slot]=false;readInFlight=false;readData.clear();snapshots[slot].clear();++generation[slot];}
        if(m.sc==0x40 && m.data.size()>=4)updateParameter(m.pid,m.data[0],m.data[1],m.data[2],m.data[3]);
    }
    void onParameterChanged(const ParameterChangeMessage& m) override {updateParameter(m.pid,m.section,m.module,m.parameter,m.value);}
    void onPatchPacketReceived(const PatchPacketMessage& m) override;
    void updateParameter(int pid,int section,int module,int param,int value){
        int matches=0;for(int slot=0;slot<4;++slot)if(patches[slot] && patchPid(slot)==pid)++matches;
        // Parameter notifications lack slot identity; never apply a shared pid
        // across multiple slots. The slot-addressed section read reconciles it.
        if(matches!=1)return;
        for(int slot=0;slot<4;++slot)if(patches[slot] && patchPid(slot)==pid){
            auto* mod=patches[slot]->getModule({section,module});auto* p=mod?mod->getParameter(param):nullptr;if(p)p->setValue(value);
        }
    }
    int patchPid(int slot) const {return confirmedPid[slot]>=0?confirmedPid[slot]:connection.getPatchId(slot);}
    void sendBoundParameter(int slot,int section,int module,int parameter,int value){
        // NME's generic ACK handler may resync its focused pid from an ACK
        // belonging to another slot. Use the pid from this slot's completed
        // patch fetch / NewPatchInSlot, never that unscoped ACK cache.
        unconfirmedWrites[slot][{section,module,parameter}]=++parameterWriteSerial;
        const int pid=patchPid(slot);auto& wire=connection.getProtocol();
        const auto focusKey=juce::String(generation[slot])+":"+juce::String(pid)+":"+juce::String(section)+":"+juce::String(module)+":"+juce::String(parameter);
        if(focusedParameter[slot]!=focusKey){ParameterFocusMessage focus;focus.pid=pid;focus.section=section;focus.module=module;focus.parameter=parameter;wire.sendMessage(NmCmd::ParameterChange,slot,focus.encode(),false,false);focusedParameter[slot]=focusKey;}
        ParameterChangeMessage change;change.pid=pid;change.section=section;change.module=module;change.parameter=parameter;change.value=value;
        wire.sendMessage(NmCmd::ParameterChange,slot,change.encode(),false,true);
    }
    juce::String binding(int slot,int index) const {
        if(!patches[slot])return {};
        const auto& k=patches[slot]->knobAssignments[index];
        return instance+":"+juce::String(generation[slot])+":"+juce::String(slot)+":"+juce::String(k.section)+":"+juce::String(k.module)+":"+juce::String(k.param);
    }
    juce::String patchBinding() const {return instance+":"+juce::String(generation[active])+":"+juce::String(active);}
    juce::String parameterBinding(int section,int module,int param) const {return patchBinding()+":"+juce::String(section)+":"+juce::String(module)+":"+juce::String(param);}
    var moduleCatalogue() const {
        if(!validSnapshot[active] || !patches[active])throw juce::String("Wait for confirmed patch");
        auto result=object();put(result,"patchBinding",patchBinding());juce::Array<var> entries;
        for(int section:{1,0})for(const auto& m:patches[active]->getContainer(section).getModules()){
            auto entry=object();put(entry,"section",section);put(entry,"index",m->getContainerIndex());put(entry,"name",m->getTitle());put(entry,"typeName",m->getDescriptor()->name);
            int count=0;for(const auto& p:m->getParameters()){auto* d=p.getDescriptor();if(d->paramClass=="parameter" && d->minValue>=0 && d->maxValue<=127)++count;}
            put(entry,"parameterCount",count);entries.add(entry);
        }
        put(result,"entries",entries);return result;
    }
    var moduleView() const {
        if(!moduleMode || !validSnapshot[active] || !patches[active])return {};
        const auto* m=patches[active]->getModule({editSection,editModule});if(!m)return {};
        auto view=object();put(view,"section",editSection);put(view,"index",editModule);put(view,"name",m->getTitle());put(view,"typeName",m->getDescriptor()->name);juce::Array<var> parameters;
        for(const auto& p:m->getParameters()){
            auto* d=p.getDescriptor();if(d->paramClass!="parameter" || d->minValue<0 || d->maxValue>127)continue;
            auto value=object();put(value,"index",d->index);put(value,"binding",parameterBinding(editSection,editModule,d->index));put(value,"name",parameterNames.name(m->getDescriptor()->index,d->index,d->maxValue,d->name));put(value,"value",p.getValue());put(value,"min",d->minValue);put(value,"max",d->maxValue);put(value,"display",ValueFormatters::format(*d,p.getValue()));parameters.add(value);
        }
        put(view,"parameters",parameters);
        const auto type=m->getDescriptor()->name;
        if(type=="NoteSeqA"||type=="NoteSeqB"||type=="CtrlSeq"||type=="EventSeq"){
            const auto* layout=meterLayouts[active].find(editSection,editModule);std::optional<int> step;
            if(layout&&layout->meterCount>0&&pendingSelectedSlot<0&&slotStage==0)step=sequencerMeters.step(active,generation[active],layout->meterBase,int64_t(juce::Time::getMillisecondCounterHiRes()));
            put(view,"playhead",step?var(*step):var());if(std::getenv("G1_KEYBOARD_TRACE")){put(view,"meterBase",layout?layout->meterBase:-1);put(view,"meterCount",layout?layout->meterCount:0);}
        }
        return view;
    }
    var valuePreview(const Parameter& parameter,int value,const juce::String& key) const {
        auto result=object();put(result,"binding",key);put(result,"value",value);put(result,"display",ValueFormatters::format(*parameter.getDescriptor(),value));return result;
    }
    var uploadState(bool bank=false) const {auto o=object();put(o,"serial",juce::int64(bank?bankSerial:uploadSerial));put(o,"busy",bank?bankBusy:uploadBusy);put(o,"ok",bank?bankOk:uploadOk);put(o,"message",problem);return o;}
    var outputControl(int slot) const {
        auto c=object();put(c,"key","slotLevel:"+juce::String(slot)+":unavailable");put(c,"label","Slot "+juce::String::charToString('A'+slot)+" · Reading patch");put(c,"value",0);put(c,"min",0);put(c,"max",0);put(c,"display","—");
        if(!validSnapshot[slot] || !patches[slot]){put(c,"label","Slot "+juce::String::charToString('A'+slot)+(slot==active?" · Reading patch":" · Select slot to read"));return c;}
        Module* output=nullptr;int section=0,count=0;
        for(int area:{1,0})for(const auto& m:patches[slot]->getContainer(area).getModules()){
            const auto type=m->getDescriptor()->name;
            if(type=="1Output"||type=="2Output"||type=="4Output"){output=m.get();section=area;++count;}
        }
        if(count!=1){put(c,"label","Slot "+juce::String::charToString('A'+slot)+(count?" · Multiple Outputs":" · No Output module"));return c;}
        const auto* p=output->getParameter(0);if(!p || p->getDescriptor()->paramClass!="parameter")return c;
        auto key="slotLevel:"+juce::String(slot)+":"+instance+":"+juce::String(generation[slot])+":"+juce::String(section)+":"+juce::String(output->getContainerIndex());
        put(c,"key",key);put(c,"label",("Slot "+juce::String::charToString('A'+slot)+" · "+patches[slot]->getName()).substring(0,64));put(c,"value",p->getValue());put(c,"min",p->getDescriptor()->minValue);put(c,"max",p->getDescriptor()->maxValue);put(c,"display",ValueFormatters::format(*p->getDescriptor(),p->getValue()));put(c,"slot",slot);put(c,"section",section);put(c,"module",output->getContainerIndex());return c;
    }
    var configuration() const {
        auto o=object();juce::Array<var> controls,selected;
        auto add=[&](const char* key,const char* label,int v,int lo,int hi){auto c=object();put(c,"key",key);put(c,"label",label);put(c,"value",v);put(c,"min",lo);put(c,"max",hi);put(c,"display",juce::String(v));controls.add(c);};
        if(settingsKnown){add("bpm","Clock tempo",settings.midiClockBpm,24,240);add("tune","Master tune",settings.masterTuneCents(),-100,100);add("sync","Global sync",settings.globalSync,0,31);add("clock","Clock source",settings.midiClockSource,0,1);add("local","Local control",settings.localOn,0,1);add("knobMode","Knob mode",settings.knobMode,0,1);add("velMin","Velocity min",settings.midiVelScaleMin,0,127);add("velMax","Velocity max",settings.midiVelScaleMax,0,127);add("pcReceive","Program receive",settings.programChangeReceive,0,1);add("pcSend","Program send",settings.programChangeSend,0,1);add("keyboard","Keyboard mode",settings.keyboardMode,0,1);add("pedal","Pedal polarity",settings.pedalPolarity,0,1);add("leds","Synth LEDs",settings.ledsActive,0,1);for(int i=0;i<4;++i){auto key="midi"+juce::String(i);add(key.toRawUTF8(),("Slot "+juce::String::charToString('A'+i)+" MIDI").toRawUTF8(),settings.midiChannelSlot[i],0,16);selected.add(connection.isSlotEnabled(i));}}
        if(mixerMode)for(int slot=0;slot<4;++slot)controls.add(outputControl(slot));
        put(o,"function",functionHeld);put(o,"controls",controls);put(o,"selectedSlots",selected);put(o,"directLink",true);put(o,"audio","Official G1-Emu VST / DAW");put(o,"model","Official G1");return o;
    }
    var status() const {
        auto o=object();const bool ready=connection.isConnected() && settingsKnown && slotKnown && validSnapshot[active] && patches[active]!=nullptr;
        put(o,"running",ready);put(o,"hostType","official-plugin");put(o,"engineAccess","pcport");put(o,"activeSlot",active);put(o,"slotKnown",connection.isConnected() && slotKnown);
        juce::Array<var> lcd,knobs;lcd.add(patches[active]?patches[active]->getName():"Waiting for official plugin");lcd.add("Official G1-Emu");put(o,"lcd",lcd);
        for(int i=0;i<18;++i){auto k=object();put(k,"index",i);put(k,"slot",active);put(k,"binding",binding(active,i));const auto* patch=patches[active].get();const auto ka=patch?patch->knobAssignments[i]:KnobAssignment{};const auto* mod=patch && ka.assigned?patch->getModule({ka.section,ka.module}):nullptr;const Parameter* p=nullptr;if(mod)for(const auto& candidate:mod->getParameters())if(candidate.getDescriptor()->index==ka.param && candidate.getDescriptor()->paramClass=="parameter"){p=&candidate;break;}put(k,"assigned",p!=nullptr);put(k,"module",mod?mod->getTitle():"");put(k,"parameter",p?parameterNames.name(mod->getDescriptor()->index,p->getDescriptor()->index,p->getDescriptor()->maxValue,p->getDescriptor()->name):"");put(k,"value",p?p->getValue():0);put(k,"max",p?p->getDescriptor()->maxValue:127);put(k,"display",p?ValueFormatters::format(*p->getDescriptor(),p->getValue()):"Unassigned");knobs.add(k);}
        if(!ready)knobs.clear();put(o,"knobs",knobs);put(o,"patchBinding",ready?patchBinding():juce::String());put(o,"edit",ready?moduleView():var());if(voices[active]>=0)put(o,"allocatedVoices",voices[active]);if(patches[active])put(o,"requestedVoices",patches[active]->getHeader().voices);
        put(o,"restore",restoreState());put(o,"upload",uploadState());put(o,"bankLoad",uploadState(true));put(o,"configuration",configuration());put(o,"padMidi",false);put(o,"midiActivity",midiActivity.snapshot());put(o,"problem",problem);return o;
    }
    var request(const var& r){
        juce::Array<var> previews;
        const auto methodName=r["method"].toString();
        if(restoreBusy&&(methodName=="set_knobs"||methodName=="set_parameters"||methodName=="set_control"||methodName=="function"))throw juce::String("Wait for slot restore to finish");
        if(methodName=="restore_slot"){beginRestore(r);auto out=object();put(out,"accepted",true);return out;}
        if(restoreBusy&&(methodName=="select_slot"||methodName=="load_file"||methodName=="load_patch"))stopRestore("Slot restore interrupted by selection/load");
        const auto method=r["method"].toString();if(method=="status")return status();
        if(method=="modules")return moduleCatalogue();
        if(method=="mixer_mode"){if(!r["enabled"].isBool())throw juce::String("Invalid mixer mode");mixerMode=bool(r["enabled"]);mixerTargets.clear();
            if(!mixerMode){for(auto i=dirtySections.begin();i!=dirtySections.end();)if(i->first!=active)i=dirtySections.erase(i);else ++i;
                if(readInFlight && readSlot!=active){readYielded=true;readDeadline=std::min(readDeadline,juce::Time::getMillisecondCounterHiRes()+50);}}
            auto o=object();put(o,"accepted",true);return o;}
        if(method=="module_exit"){moduleMode=false;parameterTargets.clear();auto out=object();put(out,"accepted",true);return out;}
        if(method=="module_select"){
            if(r["patchBinding"].toString()!=patchBinding() || !validSnapshot[active] || !patches[active])throw juce::String("Stale module selection");
            int section=integer(r,"section",0,1),index=integer(r,"index",0,127);if(!patches[active]->getModule({section,index}))throw juce::String("Unknown module");
            editSection=section;editModule=index;moduleMode=true;meterDebugFrames=0;targets.clear();parameterTargets.clear();mixerTargets.clear();auto out=object();put(out,"accepted",true);return out;
        }
        if(method=="set_parameters"){
            if(!moduleMode || !validSnapshot[active] || !patches[active] || uploadBusy || bankBusy)throw juce::String("Wait for confirmed module");
            auto* values=r["targets"].getArray();if(!values || values->size()>128)throw juce::String("Invalid parameter batch");
            std::map<std::string,ParameterTarget> checked;
            for(const auto& v:*values){int section=integer(v,"section",0,1),module=integer(v,"module",0,127),param=integer(v,"index",0,127);
                if(section!=editSection || module!=editModule || v["binding"].toString()!=parameterBinding(section,module,param))throw juce::String("Stale module parameter");
                auto* m=patches[active]->getModule({section,module});auto* p=m?m->getParameter(param):nullptr;if(!p)throw juce::String("Unknown parameter");auto* d=p->getDescriptor();if(d->minValue<0 || d->maxValue>127)throw juce::String("Unsupported parameter range");
                int value=integer(v,"value",d->minValue,d->maxValue);auto key=parameterBinding(section,module,param);previews.add(valuePreview(*p,value,key));checked[key.toStdString()]={section,module,param,value,key,juce::int64(v["inputSeq"])};
            }
            for(auto& entry:checked)parameterTargets[entry.first]=std::move(entry.second);
            auto out=object();put(out,"accepted",true);put(out,"previews",previews);return out;
        }
        if(method=="presets"){if(bool(r["refresh"]) && !connection.isFetchingPatch() && !connection.isUploadingPatch()) {listingBanks=true;connection.requestPatchList();}juce::Array<var> entries;const auto& list=connection.getPatchList();for(size_t i=0;i<list.size();++i)if(!list[i].empty()){auto e=object();put(e,"bank",int(i/99));put(e,"position",int(i%99));put(e,"name",juce::String(list[i]));entries.add(e);}auto o=object();put(o,"entries",entries);put(o,"complete",connection.isPatchListLoaded());return o;}
        if(method=="set_knobs"){
            if(functionHeld){auto result=object();put(result,"accepted",true);return result;}
            if(!validSnapshot[active] || !patches[active] || uploadBusy || bankBusy)throw juce::String("Wait for confirmed patch");auto* values=r["targets"].getArray();if(!values || values->size()>18)throw juce::String("Invalid knob batch");
            for(const auto& v:*values){int i=integer(v,"index",0,17);if(v["binding"].toString()!=binding(active,i))throw juce::String("Stale parameter binding");const auto& ka=patches[active]->knobAssignments[i];auto* m=ka.assigned?patches[active]->getModule({ka.section,ka.module}):nullptr;auto* p=m?m->getParameter(ka.param):nullptr;if(!p)throw juce::String("Unassigned knob");int value=integer(v,"value",p->getDescriptor()->minValue,p->getDescriptor()->maxValue);previews.add(valuePreview(*p,value,binding(active,i)));targets[i]=value;inputSequences[i]=juce::int64(v["inputSeq"]);
                if(pipelineTrace())std::fprintf(stderr,"PIPELINE ts_ms=%lld stage=adapter_target input_seq=%lld knob=%d value=%d\n",(long long)juce::Time::currentTimeMillis(),(long long)inputSequences[i],i,value);
            }
        } else if(method=="select_slot"){
            targets.clear();parameterTargets.clear();mixerTargets.clear();const int slot=integer(r,"slot",0,3);
            if(functionHeld){if(!settingsKnown || !settings.hasExtended)throw juce::String("Wait for confirmed slot state");auto desired=pinned;desired[slot]=!desired[slot];if(!connection.setSlotPins(desired))throw juce::String("Wait for confirmed slot state");pinned=desired;}
            else {
                if(!settingsKnown || !settings.hasExtended)throw juce::String("Wait for confirmed slot state");
                // Same PC Port mask/activation payloads as NME selectSlot. The synth
                // confirms these via NMInfo; do not stall its following command on
                // NmProtocol's genuine-reply-only global wait. Actual state still
                // comes exclusively from SlotActivated / SynthSettings readback.
                const auto now=juce::Time::getMillisecondCounterHiRes();
                sequencerMeters.reset();meterStreamSeen=false;pendingSelectedSlot=slot;slotTarget=slot;slotStage=1;slotSendAt=std::max(now,lastSlotWriteAt+300.0);
                slotConfirmDeadline=now+1800;nextSettings=0;
                if(readInFlight)readDeadline=std::min(readDeadline,now+150.0);
                if(pipelineTrace())std::fprintf(stderr,"SLOT_REQUEST target=%d active=%d\n",slot,active);
            }
        }
        else if(method=="load_file"){
            if(connection.isFetchingPatch() || connection.isUploadingPatch())throw juce::String("Wait for the current patch operation");int slot=integer(r,"slot",0,3);juce::File file(r["path"].toString());if(!file.hasFileExtension("pch") || !file.existsAsFile() || file.getSize()>4*1024*1024)throw juce::String("Invalid patch file");PchFileIO io(descriptions);auto patch=io.readFile(file);if(!patch)throw juce::String("Cannot parse patch");if(const auto refused=connection.firstModuleRefused(*patch);refused.isNotEmpty())throw juce::String("Patch requires unsupported module: ")+refused;if(deferredPatch || uploadBusy || bankBusy)throw juce::String("Wait for the current patch operation");
            targets.clear();parameterTargets.clear();mixerTargets.clear();dirtySections.clear();readAt=0;fetchAfterUpload=false;
            uploadSlot=slot;uploadBusy=true;uploadOk=false;++uploadSerial;problem={};
            deferredPatch=std::move(patch);
        } else if(method=="load_patch"){uploadSlot=integer(r,"slot",0,3);targets.clear();parameterTargets.clear();mixerTargets.clear();bankBusy=true;bankOk=false;++bankSerial;connection.loadPatchFromBank(integer(r,"bank",0,8),integer(r,"position",0,98),uploadSlot);}
        else if(method=="function"){if(!r["pressed"].isBool())throw juce::String("Invalid Function state");if(bool(r["pressed"]) && !functionHeld)for(int i=0;i<4;++i)if(i!=active)pinned[i]=connection.isSlotEnabled(i);functionHeld=bool(r["pressed"]);targets.clear();parameterTargets.clear();mixerTargets.clear();dirtySections.clear();}
        else if(method=="pad_panic"){} // Keybed MIDI is deliberately independent.
        else if(method=="show_settings")throw juce::String("Open Settings in the official G1-Emu plugin editor");
        else if(method=="set_control"){
            const auto key=r["key"].toString();
            if(key.startsWith("slotLevel:")){
                if(!mixerMode)throw juce::String("Mixer is not active");
                for(int slot=0;slot<4;++slot){auto c=outputControl(slot);if(c["key"].toString()!=key)continue;
                    if(int(c["max"])==0)throw juce::String("No single Output level available");
                    int value=integer(r,"value",int(c["min"]),int(c["max"]));int section=int(c["section"]),module=int(c["module"]);
                    mixerTargets[slot]={slot,section,module,value,generation[slot]};
                    auto* p=patches[slot]->getModule({section,module})->getParameter(0);auto out=object();juce::Array<var> accepted;accepted.add(valuePreview(*p,value,key));put(out,"accepted",true);put(out,"previews",accepted);return out;
                }
                throw juce::String("Stale mixer target");
            }
            if(!settingsKnown || connection.isFetchingPatch() || connection.isUploadingPatch() || readInFlight)throw juce::String("Wait for confirmed synth settings");
auto config=configuration();const auto* controls=config["controls"].getArray();const var* found=nullptr;for(const auto& c:*controls)if(c["key"].toString()==key)found=&c;if(!found)throw juce::String("Unsupported official setting");
            int value=integer(r,"value",int((*found)["min"]),int((*found)["max"]));auto updated=settings;
            if(key=="bpm")updated.midiClockBpm=value;else if(key=="tune")updated.setMasterTuneCents(value);else if(key=="sync")updated.globalSync=value;else if(key=="clock")updated.midiClockSource=value;else if(key=="local")updated.localOn=value;else if(key=="knobMode")updated.knobMode=value;else if(key=="velMin")updated.midiVelScaleMin=value;else if(key=="velMax")updated.midiVelScaleMax=value;else if(key=="pcReceive")updated.programChangeReceive=value;else if(key=="pcSend")updated.programChangeSend=value;else if(key=="keyboard")updated.keyboardMode=value;else if(key=="pedal")updated.pedalPolarity=value;else if(key=="leds")updated.ledsActive=value;else if(key.startsWith("midi"))updated.midiChannelSlot[key.substring(4).getIntValue()]=value;
            connection.sendSynthSettings(updated);nextSettings=juce::Time::getMillisecondCounterHiRes()+350;
        }
        else throw juce::String("Unsupported official-plugin command");
        auto o=object();put(o,"accepted",true);if(!previews.isEmpty())put(o,"previews",previews);return o;
    }
    void tick(){
        midiActivity.refresh();
        if(!connection.hasOpenPorts()){stopping=true;return;}
        if(!client && server.waitUntilReady(true,0)>0){client.reset(server.waitForNextConnection());if(client){int one=1;::setsockopt(client->getRawSocketHandle(),SOL_SOCKET,SO_NOSIGPIPE,&one,sizeof(one));auto h=object();put(h,"service","modular-machine-g1");put(h,"protocol",1);put(h,"instance",instance);put(h,"hostType","official-plugin");send(h);}}
        if(client && client->waitUntilReady(true,0)>0){char b[8192];int count=client->read(b,sizeof(b),false);if(count<=0){client.reset();input.clear();output.clear();targets.clear();parameterTargets.clear();mixerTargets.clear();functionHeld=false;moduleMode=false;mixerMode=false;slotStage=0;pendingSelectedSlot=-1;}else input.append(b,size_t(count));}
        for(int n=0;client && n<8;++n){auto end=input.find('\n');if(end==std::string::npos)break;auto r=juce::JSON::parse(juce::String::fromUTF8(input.data(),int(end)));input.erase(0,end+1);auto out=object();put(out,"id",r["id"]);try{put(out,"result",request(r));}catch(const juce::String& e){auto rejected=object();put(rejected,"rejected",e);put(out,"result",rejected);}send(out);}
        if(input.size()>1024*1024){client.reset();input.clear();output.clear();targets.clear();parameterTargets.clear();mixerTargets.clear();functionHeld=false;moduleMode=false;mixerMode=false;slotStage=0;pendingSelectedSlot=-1;}
        if(client && !output.empty()){
            auto written=::send(client->getRawSocketHandle(),output.data(),std::min<size_t>(output.size(),65536),MSG_DONTWAIT);
            if(written>0)output.erase(0,size_t(written));else if(written<0 && errno!=EAGAIN && errno!=EWOULDBLOCK && errno!=EINTR){client.reset();output.clear();targets.clear();parameterTargets.clear();mixerTargets.clear();functionHeld=false;moduleMode=false;mixerMode=false;slotStage=0;pendingSelectedSlot=-1;}
        }
        auto now=juce::Time::getMillisecondCounterHiRes();
        advanceRestore(now);
        if(pendingSelectedSlot>=0 && now>=slotConfirmDeadline){pendingSelectedSlot=-1;slotStage=0;}
        // The official SynthSettingsLink uses a 300ms quiet interval between
        // slot writes: the ROM drops SlotActivated sent immediately after mask.
        if(slotStage>0 && now>=slotSendAt && connection.isConnected() && !readInFlight && !connection.isFetchingPatch() && !connection.isUploadingPatch() && !connection.getProtocol().isWaitingForReply()){
            if(slotStage==1){
                int mask=0;for(int i=0;i<4;++i)if(pinned[i]||i==slotTarget)mask|=1<<(3-i);
                connection.getProtocol().sendMessage(0x17,0,{0x41,0x07,static_cast<uint8_t>(mask)},false,true);
                slotStage=2;slotSendAt=now+300;
            }else{
                connection.getProtocol().sendMessage(0x17,0,{0x41,0x09,static_cast<uint8_t>(slotTarget)},false,true);
                slotStage=0;nextSettings=now+300;
            }
            lastSlotWriteAt=now;
        }
        if(nextSettings>0 && connection.isConnected() && now>=nextSettings && !readInFlight && !deferredPatch && !connection.isFetchingPatch() && !connection.isUploadingPatch() && !connection.getProtocol().isWaitingForReply()){
            RequestSynthSettingsMessage query;
            connection.getProtocol().sendMessage(0x17,0,query.encode(),false,true);
            nextSettings=pendingSelectedSlot>=0 && now<slotConfirmDeadline?now+150:(settingsKnown?0:now+1000);
        }
        // A confirmed focus change may arrive in settings without an SC09
        // echo. NME then changes its current slot but does not fetch that patch.
        if(connection.isConnected() && slotKnown && slotStage==0 && pendingSelectedSlot<0 && !validSnapshot[active] && now>=nextSlotFetch && !readInFlight && !deferredPatch && !connection.isFetchingPatch() && !connection.isUploadingPatch() && !connection.getProtocol().isWaitingForReply()){
            nextSlotFetch=now+1000;connection.requestPatch(active);
        }
        // Mixer consumes already-confirmed slot models. Never launch full
        // inactive-slot fetches from a display page: they can monopolize the
        // PC Port and delay a user's edits or slot activation. Selecting a slot
        // uses the normal active-slot fetch path above.
        const bool wireWait=connection.getProtocol().isWaitingForReply();
        if(wireWait && waitSince==0)waitSince=now;
        if(!wireWait)waitSince=0;
        if((!targets.empty() || uploadBusy || bankBusy) && now>=nextDiagnostic &&
            ((waitSince>0 && now-waitSince>250) || deferredPatch || readInFlight)){
            std::fprintf(stderr,"CONTROL_WAIT wait_ms=%.0f read=%d fetch=%d upload=%d deferred=%d targets=%zu\n",
                waitSince>0?now-waitSince:0,readInFlight,connection.isFetchingPatch(),connection.isUploadingPatch(),bool(deferredPatch),targets.size());
            nextDiagnostic=now+1000;
        }
        if(readInFlight && readSlot!=active && (!targets.empty() || !parameterTargets.empty())){
            readYielded=true;readDeadline=std::min(readDeadline,now+50);
        }
        if(!targets.empty() || !parameterTargets.empty() || !mixerTargets.empty()){connection.cancelPatchListFetch("Keyboard editing takes priority");listingBanks=false;}
        // Live edits have priority over background parameter snapshots, as in
        // the standalone transport fix. A due/streaming read must not gate input.
        if(slotStage==0 && pendingSelectedSlot<0 && now>=lastSlotWriteAt+300 && now>=nextWrite && (!targets.empty() || !parameterTargets.empty() || !mixerTargets.empty()) && !connection.isFetchingPatch() && !connection.isUploadingPatch() && !connection.getProtocol().isWaitingForReply()){
            for(auto [i,v]:targets){
                if(!patches[active])break;
                const auto& k=patches[active]->knobAssignments[i];
                sendBoundParameter(active,k.section,k.module,k.param,v);
                if(pipelineTrace())std::fprintf(stderr,"PIPELINE ts_ms=%lld stage=protocol_send input_seq=%lld knob=%d value=%d\n",(long long)juce::Time::currentTimeMillis(),(long long)inputSequences[i],i,v);
                if(k.section<2)dirtySections.insert({active,k.section});
            }
            for(const auto& item:parameterTargets){const auto& p=item.second;
                if(p.binding!=parameterBinding(p.section,p.module,p.param))continue;
                sendBoundParameter(active,p.section,p.module,p.param,p.value);dirtySections.insert({active,p.section});
                if(pipelineTrace())std::fprintf(stderr,"PIPELINE ts_ms=%lld stage=protocol_send input_seq=%lld module=%d param=%d value=%d\n",(long long)juce::Time::currentTimeMillis(),(long long)p.input,p.module,p.param,p.value);
            }
            for(const auto& entry:mixerTargets){const auto& t=entry.second;if(t.generation!=generation[t.slot]||!validSnapshot[t.slot])continue;
                sendBoundParameter(t.slot,t.section,t.module,0,t.value);dirtySections.insert({t.slot,t.section});}
            targets.clear();parameterTargets.clear();mixerTargets.clear();nextWrite=now+16;
            // Preserve the first readback deadline during a continuous turn.
            // Debouncing this on every write starves confirmed display updates.
            if(readAt==0)readAt=now+16;
        }
        if(readInFlight && now>=readDeadline){
            readInFlight=false;readData.clear();const auto key=std::make_pair(readSlot,readSection);
            if(readGeneration==generation[readSlot]){
                const bool wanted=readSlot==active || mixerMode;
                const bool retry=readYielded || ++readFailures[key]<2;
                if(wanted && retry){dirtySections.insert(key);if(readAt==0)readAt=now+16;}
                else if(wanted && !readYielded){if(restoreBusy&&readSlot==restoreSlot)stopRestore("Slot restore readback timed out; press CLEAR to retry");validSnapshot[readSlot]=false;dirtySections.erase(key);mixerTargets.erase(readSlot);if(readSlot==active){targets.clear();parameterTargets.clear();}std::fprintf(stderr,"READBACK_REFRESH slot=%d section=%d expected_pid=%d\n",readSlot,readSection,patchPid(readSlot));}
            }
            readYielded=false;
        }
        if(deferredPatch && !readInFlight && !connection.isFetchingPatch() && !connection.isUploadingPatch() && !connection.getProtocol().isWaitingForReply()){
            dirtySections.clear();readAt=0;readData.clear();
            connection.uploadPatch(uploadSlot,*deferredPatch);deferredPatch.reset();
            connection.setUploadCompleteCallback([this]{readAt=juce::Time::getMillisecondCounterHiRes()+100;fetchAfterUpload=true;});
        }
        if(slotStage==0 && pendingSelectedSlot<0 && now>=lastSlotWriteAt+300 && readAt>0 && now>=readAt && now>=nextParameterRead && (!restoreBusy||restoreQueue.empty()) && targets.empty() && parameterTargets.empty() && mixerTargets.empty() && !readInFlight && !connection.isFetchingPatch() && !connection.isUploadingPatch() && !connection.getProtocol().isWaitingForReply()){
            readAt=0;
            if(fetchAfterUpload){fetchAfterUpload=false;connection.requestPatch(uploadSlot);}
            else if(!dirtySections.empty()){
                nextParameterRead=now+40;
                auto next=std::find_if(dirtySections.begin(),dirtySections.end(),[this](const auto& p){return p.first==active;});if(next==dirtySections.end())next=dirtySections.begin();
                readSlot=next->first;readSection=next->second;dirtySections.erase(next);readGeneration=generation[readSlot];readWriteSerial=parameterWriteSerial;readData.clear();readInFlight=true;readYielded=false;readStartedAt=now;readDeadline=now+(readSlot==active?500:250);
                if(pipelineTrace())std::fprintf(stderr,"PIPELINE ts_ms=%lld stage=read_request slot=%d section=%d\n",(long long)juce::Time::currentTimeMillis(),readSlot,readSection);
                GetPatchMessage query;query.pid=patchPid(readSlot);query.section=readSection==1?GetPatchMessage::PolyParameter:GetPatchMessage::CommonParameter;
                // This adapter owns the read deadline/assembly. Do not also lock
                // NME's global send queue for up to its 3-second reply timeout.
                // Keep SDK full-patch operations serialized by their existing guards.
                connection.getProtocol().sendMessage(0x17,readSlot,query.encode(),false,true);
            }
        }
    }
    void send(const var& o){if(!client)return;auto data=juce::JSON::toString(o,true)+"\n";if(output.size()+size_t(data.getNumBytesAsUTF8())>2*1024*1024){client.reset();output.clear();return;}output.append(data.toRawUTF8(),size_t(data.getNumBytesAsUTF8()));}
private:
    ThemeData meterTheme;std::array<LightMeterLayout::Table,4> meterLayouts;SequencerMeters sequencerMeters;bool meterStreamSeen=false;int meterDebugFrames=0;
    std::array<OriginalValues,4> originals;std::array<int,4> originalPid{-1,-1,-1,-1};std::array<uint64_t,4> originalGeneration{};
    std::array<std::map<ParameterKey,uint64_t>,4> unconfirmedWrites;uint64_t parameterWriteSerial=0,readWriteSerial=0;
    bool restoreBusy=false;int restoreSlot=0,restoreWritten=0,restoreTotal=0;uint64_t restoreGeneration=0,restoreSerial=0;double nextRestoreWrite=0,restoreDeadline=0;
    std::deque<ParameterKey> restoreQueue;std::set<int> restoreSections;juce::String restoreMessage;
    MidiActivity midiActivity;
    ParameterNames parameterNames;
    struct ParameterTarget{int section,module,param,value;juce::String binding;juce::int64 input;};
    std::map<std::string,ParameterTarget> parameterTargets;
    struct MixerTarget{int slot,section,module,value;uint64_t generation;};std::map<int,MixerTarget> mixerTargets;
    bool mixerMode=false;
    bool moduleMode=false;int editSection=1,editModule=0;
    std::array<int,4> confirmedPid{-1,-1,-1,-1};std::array<juce::String,4> focusedParameter;
    std::array<bool,4> pinned{};bool functionHeld=false,listingBanks=false;
    int directPort,jsonPort,active=0,uploadSlot=0,pendingSelectedSlot=-1,slotTarget=0,slotStage=0;double slotConfirmDeadline=0,slotSendAt=0,lastSlotWriteAt=0,nextSlotFetch=0;bool settingsKnown=false,slotKnown=false,uploadBusy=false,uploadOk=false,bankBusy=false,bankOk=false;
    uint64_t uploadSerial=0,bankSerial=0;std::array<uint64_t,4> generation{};std::array<juce::String,4> fingerprints;int voices[4]={-1,-1,-1,-1};
    juce::String instance=juce::Uuid().toString(),problem;SynthSettings settings;
    ModuleDescriptions descriptions;PatchParser parser;ConnectionManager connection;
    std::array<bool,4> incomplete{},validSnapshot{};std::array<std::unique_ptr<Patch>,4> patches;std::array<std::vector<std::vector<uint8_t>>,4> snapshots;
    std::unique_ptr<Patch> deferredPatch;
    std::map<int,juce::int64> inputSequences;
    std::map<std::pair<int,int>,int> readFailures;
    std::map<int,int> targets;bool readYielded=false;double readStartedAt=0,nextMismatchLog=0;bool readInFlight=false,fetchAfterUpload=false;int readSlot=0,readSection=0;uint64_t readGeneration=0;double readDeadline=0;std::vector<uint8_t> readData;std::set<std::pair<int,int>> dirtySections;double waitSince=0,nextDiagnostic=0,nextWrite=0,readAt=0,nextParameterRead=0,nextSettings=0;
    juce::StreamingSocket server;std::unique_ptr<juce::StreamingSocket> client;std::string input,output;
};
void Adapter::onPatchPacketReceived(const PatchPacketMessage& m) {
    if(!readInFlight || connection.isFetchingPatch())return;
    if(m.pid!=patchPid(readSlot)){
        const auto now=juce::Time::getMillisecondCounterHiRes();if(pipelineTrace() && now>=nextMismatchLog){std::fprintf(stderr,"READBACK_PID_MISMATCH slot=%d expected=%d received=%d\n",readSlot,patchPid(readSlot),m.pid);nextMismatchLog=now+1000;}return;
    }
    if(m.isFirst)readData.clear();
    if(readData.size()+m.patchData.size()>1024*1024){readInFlight=false;readData.clear();return;}
    readData.insert(readData.end(),m.patchData.begin(),m.patchData.end());
    if(!m.isLast)return;
    readInFlight=false;
    if(pipelineTrace())std::fprintf(stderr,"PIPELINE ts_ms=%lld stage=read_response slot=%d section=%d bytes=%zu elapsed_ms=%.3f\n",(long long)juce::Time::currentTimeMillis(),readSlot,readSection,readData.size(),juce::Time::getMillisecondCounterHiRes()-readStartedAt);
    if(readGeneration!=generation[readSlot] || !patches[readSlot]){readData.clear();return;}
    try {
        BitStream bits(readData);if(bits.readBits(8)!=77 || int(bits.readBits(1))!=readSection){readData.clear();return;}
        auto sections=snapshots[readSlot];bool found=false;
        for(auto& section:sections){BitStream old(section);if(old.readBits(8)==77 && int(old.readBits(1))==readSection){section=readData;found=true;break;}}
        if(found){auto patch=parser.parse(sections);if(patch){readFailures.erase({readSlot,readSection});patches[readSlot]=std::move(patch);snapshots[readSlot]=std::move(sections);problem={};
            const auto actual=parameterValues(readSlot);
            for(auto i=unconfirmedWrites[readSlot].begin();i!=unconfirmedWrites[readSlot].end();){
                const auto baseline=originals[readSlot].find(i->first);const auto value=actual.find(i->first);
                if(std::get<0>(i->first)==readSection&&i->second<=readWriteSerial&&baseline!=originals[readSlot].end()&&value!=actual.end()&&baseline->second.value==value->second.value)i=unconfirmedWrites[readSlot].erase(i);else ++i;
            }
            confirmRestore(readSlot,readSection);}}
    }catch(const std::exception& e){problem=juce::String("Invalid official parameter response: ")+e.what();}
    readData.clear();if(!dirtySections.empty())readAt=juce::Time::getMillisecondCounterHiRes()+16;
}
// Own AppKit once, as in the standalone controller freeze/latency fix.
// A coalesced native clock wakes the message thread; protocol/model ownership
// stays on that thread. Never queue one task per knob event or reenter AppKit.
class AdapterPump final : private juce::HighResolutionTimer, private juce::AsyncUpdater {
public:
    explicit AdapterPump(Adapter& adapter):adapter(adapter){startTimer(2);}
    ~AdapterPump() override {stopTimer();cancelPendingUpdate();}
private:
    void hiResTimerCallback() override {triggerAsyncUpdate();}
    void handleAsyncUpdate() override {
        if(stopping){
#if JUCE_MAC
            CFRunLoopStop(CFRunLoopGetMain());
#else
            juce::MessageManager::getInstance()->stopDispatchLoop();
#endif
            return;
        }
        adapter.tick();
    }
    Adapter& adapter;
};
int main(int argc,char** argv){
    if(argc!=3){std::fprintf(stderr,"usage: g1keyboardadapter DIRECT_PORT JSON_PORT\n");return 2;}
    std::signal(SIGINT,stop);std::signal(SIGTERM,stop);juce::ScopedJuceInitialiser_GUI gui;juce::Process::setDockIconVisible(false);
    if(!std::getenv("G1_KEYBOARD_TRACE"))std::cout.setstate(std::ios_base::failbit);
    try{Adapter adapter(std::atoi(argv[1]),std::atoi(argv[2]));std::puts("OFFICIAL ADAPTER READY");std::fflush(stdout);AdapterPump pump(adapter);
#if JUCE_MAC
        // The windowless console adapter has no NSApplication::run lifecycle.
        // JUCE's async message source is serviced directly by the main CF loop.
        CFRunLoopRun();
#else
        juce::MessageManager::getInstance()->runDispatchLoop();
#endif
    }
    catch(const juce::String& e){std::fprintf(stderr,"%s\n",e.toRawUTF8());return 1;}
}
