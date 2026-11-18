// Local G1-Emu controller bridge. GPL-3.0-or-later.
// Reuses G1-Emu's own panel/PresetsLink; never takes the editor's DirectLink socket.
#pragma once
#include "emuhost.h"
#include "PchUpload.h"
#include "LcdText.h"
#include "g1Lib/g1knobs.h"
#include "g1Lib/g1format.h"
#include <juce_gui_basics/juce_gui_basics.h>
#include <algorithm>
#include <cerrno>
#include <cmath>
#include <cstdlib>
#include <memory>
#include <string>
#include <vector>
#include <functional>
#include <optional>
#include <sstream>
#include <future>
#include <deque>
#include <mutex>
#include <thread>
#include <atomic>
#include <chrono>
#include <type_traits>
#if JUCE_MAC || JUCE_LINUX
#include <sys/socket.h>
#endif

namespace g1app {
template<class Host> class BasicMk3ControlBridge {
public:
    class SettingsCallback {
    public:
        void operator=(std::function<void()> callback){std::lock_guard<std::mutex> lock(gate);value=std::move(callback);}
        explicit operator bool() const {return static_cast<bool>(get());}
        std::function<void()> get() const {std::lock_guard<std::mutex> lock(gate);return value;}
    private:
        mutable std::mutex gate;std::function<void()> value;
    };
    SettingsCallback onShowSettings;
    explicit BasicMk3ControlBridge(Host& host, int port) : host(host) {
        if (port < 1024 || port > 65535) return;
        if (!listener.createListener(port, "127.0.0.1")) {
            std::fprintf(stderr, "MK3 bridge: cannot bind 127.0.0.1:%d\n", port); return;
        }
        listeningPort=port;
        std::fprintf(stderr, "MK3 bridge v1 on 127.0.0.1:%d\n", port);
        host.synthSettings().setReread(2000);
        // Socket progress must not depend on JUCE's UI message thread. All synth
        // access still goes through the host's bounded engine-access queue below.
        transport=std::thread([this]{
            while(!stopping.load(std::memory_order_acquire)) {
                pollTransport();
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
        });
    }
    ~BasicMk3ControlBridge() {
        stopping.store(true,std::memory_order_release);if(transport.joinable())transport.join();closeClient();
        std::optional<EmuHost::Options> settings;
        {std::lock_guard<std::mutex> lock(lifetime->gate);lifetime->alive=false;settings=std::move(settingsToSave);}
        if(settings)saveSettingsSnapshot(*settings);
    }
    bool listening() const {return listeningPort>0;}
    static int configuredPort() {
        const auto* value = std::getenv("G1_MK3_PORT");
        if (!value) return 0;
        char* end = nullptr; const long port = std::strtol(value, &end, 10);
        return end != value && *end == '\0' && port >= 1024 && port <= 65535 ? static_cast<int>(port) : 0;
    }
private:
    static void saveSettingsSnapshot(const EmuHost::Options& options) {
        if constexpr(std::is_same_v<Host,EmuHost>)options.save(EmuHost::defaultSettingsPath());
    }
    void hostInfo(juce::var& value) {
        if constexpr(std::is_same_v<Host,EmuHost>)put(value,"hostType","standalone");
        else host.bridgeInfo(value);
    }
    static juce::var object() { return juce::var(new juce::DynamicObject()); }
    static void put(juce::var& value, const char* key, const juce::var& field) { value.getDynamicObject()->setProperty(key, field); }
    static int integer(const juce::var& request, const char* key, int min, int max) {
        auto v = request.getProperty(key, {});
        if ((!v.isInt() && !v.isInt64()) || static_cast<juce::int64>(v) < min || static_cast<juce::int64>(v) > max)
            throw juce::String("Invalid integer: ") + key;
        return static_cast<int>(v);
    }
    std::string binding(const g1::KnobInfo& k) const {
        const auto epoch=[&]{if constexpr(std::is_same_v<Host,EmuHost>)return std::string();else return host.bindingEpoch()+":";}();
        return epoch+std::to_string(host.presets().patchId(k.slot))+":"+std::to_string(k.slot)+":"+std::to_string(k.section)+":"+std::to_string(k.module)+":"+std::to_string(k.param)+":"+std::to_string(k.type)+":"+std::to_string(k.max)+":"+k.moduleName+":"+k.paramName;
    }
    static juce::String clean(const std::string& text) {
        return juce::String::fromUTF8(lcdText(text).c_str());
    }
    bool ready() {
        SynthSettings settings; uint64_t revision=0;
        return host.running() && host.synthSettings().settings(settings,revision);
    }
    juce::var control(const char* key,const char* label,int value,int min,int max,const juce::String& display) {
        auto c=object();put(c,"key",key);put(c,"label",label);put(c,"value",value);put(c,"min",std::min(min,value));put(c,"max",std::max(max,value));put(c,"display",display);return c;
    }
    juce::var configuration() {
        auto result=object();juce::Array<juce::var> controls;
        if(!ready()){put(result,"controls",juce::var(controls));return result;}
        SynthSettings s;uint64_t revision=0;host.synthSettings().settings(s,revision);
        if(pendingSettings && (s==*pendingSettings || juce::Time::getMillisecondCounterHiRes()>settingsUntil))pendingSettings.reset();
        put(result,"audio",juce::String(host.stats().audio));put(result,"rom",juce::String(host.options().romInUse));
        put(result,"model",host.options().expanded?"Expanded":"Regular");put(result,"directLink",host.options().directLink);
        juce::Array<juce::var> selected;for(bool on:s.slotSelected)selected.add(on);put(result,"selectedSlots",juce::var(selected));
        put(result,"function",functionHeld);put(result,"settingsPending",pendingSettings.has_value());
        controls.add(control("master","Master volume",host.mc().adc(0x30),0,255,juce::String(juce::roundToInt(host.mc().adc(0x30)*100.0/255.0))+"%"));
        const int gain=juce::roundToInt(host.options().gainDb);
        controls.add(control("gain","Output gain",gain,0,60,juce::String(gain)+" dB"));
        controls.add(control("bpm","Clock tempo",s.clockBpm,24,240,juce::String(s.clockBpm)+" BPM"));
        controls.add(control("tune","Master tune",s.masterTune,-100,100,juce::String(s.masterTune)+" ct"));
        controls.add(control("sync","Global sync",s.globalSync,0,31,juce::String(s.globalSync+1)+" QN"));
        for(int i=0;i<4;++i){const auto key=std::string("midi")+std::to_string(i);const auto label=std::string("Slot ")+char('A'+i)+" MIDI";
            controls.add(control(key.c_str(),label.c_str(),s.midiChannel[i],0,16,s.midiChannel[i]==16?"Off":juce::String(s.midiChannel[i]+1)));}
        controls.add(control("clock","Clock source",s.clockInternal?1:0,0,1,s.clockInternal?"Internal":"External"));
        controls.add(control("local","Local control",s.localOn?1:0,0,1,s.localOn?"On":"Off"));
        controls.add(control("knobMode","Knob mode",s.knobMode,0,1,s.knobMode?"Hook":"Immediate"));
        controls.add(control("velMin","Velocity min",s.velScaleMin,0,127,juce::String(s.velScaleMin)));
        controls.add(control("velMax","Velocity max",s.velScaleMax,0,127,juce::String(s.velScaleMax)));
        controls.add(control("pcReceive","Program receive",s.programChangeReceive?1:0,0,1,s.programChangeReceive?"On":"Off"));
        controls.add(control("pcSend","Program send",s.programChangeSend?1:0,0,1,s.programChangeSend?"On":"Off"));
        controls.add(control("keyboard","Keyboard mode",s.keyboardMode,0,1,s.keyboardMode?"Selected slots":"Active slot"));
        controls.add(control("pedal","Pedal polarity",s.pedalPolarity,0,1,s.pedalPolarity?"Inverted":"Normal"));
        controls.add(control("leds","Synth LEDs",s.ledsActive?1:0,0,1,s.ledsActive?"On":"Off"));
        put(result,"controls",juce::var(controls));return result;
    }
    void scheduleSettingsSave() {
        settingsToSave=host.options();
        if(settingsSaveScheduled)return;
        settingsSaveScheduled=true;auto life=lifetime;
        juce::MessageManager::callAsync([this,life]{
            juce::Timer::callAfterDelay(250,[this,life]{
                std::optional<EmuHost::Options> settings;
                {std::lock_guard<std::mutex> lock(life->gate);if(!life->alive)return;
                 settings=std::move(settingsToSave);settingsToSave.reset();settingsSaveScheduled=false;}
                if(settings)saveSettingsSnapshot(*settings);
            });
        });
    }
    void setControl(const juce::var& request) {
        const auto key=request.getProperty("key",{}).toString();auto config=configuration();
        const auto* controls=config["controls"].getArray();const juce::var* found=nullptr;
        for(const auto& c:*controls)if(c["key"].toString()==key)found=&c;
        if(!found)throw juce::String("Unknown real emulator setting");
        const int value=integer(request,"value",static_cast<int>((*found)["min"]),static_cast<int>((*found)["max"]));
        if(key=="master"){host.mc().setAdc(0x30,static_cast<uint8_t>(value));return;}
        if(key=="gain"){host.setGainDb(static_cast<float>(value));if constexpr(std::is_same_v<Host,EmuHost>){if(onShowSettings)scheduleSettingsSave();}return;}
        SynthSettings s;uint64_t revision=0;if(!host.synthSettings().settings(s,revision))throw juce::String("Synth settings unavailable");
        if(pendingSettings)s=*pendingSettings;
        if(key=="bpm")s.clockBpm=value;else if(key=="tune")s.masterTune=value;else if(key=="sync")s.globalSync=value;
        else if(key.startsWith("midi"))s.midiChannel[static_cast<size_t>(key.substring(4).getIntValue())]=value;
        else if(key=="clock")s.clockInternal=value!=0;else if(key=="local")s.localOn=value!=0;
        else if(key=="knobMode")s.knobMode=value;else if(key=="velMin")s.velScaleMin=value;else if(key=="velMax")s.velScaleMax=value;
        else if(key=="pcReceive")s.programChangeReceive=value!=0;else if(key=="pcSend")s.programChangeSend=value!=0;
        else if(key=="keyboard")s.keyboardMode=value;else if(key=="pedal")s.pedalPolarity=value;else if(key=="leds")s.ledsActive=value!=0;
        if(s.velScaleMin>s.velScaleMax)throw juce::String("Velocity minimum must not exceed maximum");
        pendingSettings=s;settingsUntil=juce::Time::getMillisecondCounterHiRes()+5000.;host.synthSettings().write(s);
    }
    juce::var status() {
        auto result = object(); put(result,"running",ready());hostInfo(result);
        put(result,"encoderInput","editor-parameters");put(result,"engineThreadRequests",std::is_same_v<Host,EmuHost>);put(result,"transportThread","dedicated");put(result,"problem",juce::String(host.stats().dspProblem));
        put(result,"configuration",configuration());
        put(result,"padMidi",true);put(result,"midiInBytes",static_cast<juce::int64>(host.stats().midiIn));
        juce::Array<juce::var> held;for(int i=0;i<16;++i)if(heldPads[static_cast<size_t>(i)].channel>=0)held.add(i);
        put(result,"padMidiHeld",juce::var(held));
        const auto state=host.presets().uploadStatus();auto upload=object();
        put(upload,"serial",static_cast<juce::int64>(state.serial));put(upload,"busy",state.busy);
        put(upload,"ok",state.ok);put(upload,"message",juce::String(state.message));put(result,"upload",upload);
        const auto loaded=host.presets().loadStatus();auto bank=object();
        put(bank,"serial",static_cast<juce::int64>(loaded.serial));put(bank,"busy",loaded.busy);put(bank,"ok",loaded.ok);put(bank,"message",juce::String(loaded.message));put(result,"bankLoad",bank);
        put(result,"activeSlot",0); put(result,"lcd",juce::var(juce::Array<juce::var>{"", ""}));
        juce::Array<juce::var> knobs;
        if (ready()) {
            if(host.options().dspCount()<g1::g_regularDsps)throw juce::String("Initial MK3 bridge supports Regular/Expanded G1, not Micro");
            auto& mc = host.mc(); g1::KnobMap map(mc);
            const int active=static_cast<int>(mc.read8(g1::KnobMap::ActiveSlot+map.osShift())&3);put(result,"activeSlot",active);
            const auto now=juce::Time::getMillisecondCounterHiRes();
            if(!state.busy && (active!=voiceSlot || now>=voiceRefresh)) {host.presets().readVoices(active);voiceSlot=active;voiceRefresh=now+1000.;}
            auto line=mc.getLcd().line(1);for(char& c:line)if(c=='(' || c==')')c=' ';
            std::istringstream stream(line);std::array<int,4> counts{};bool valid=true;std::string token;
            for(auto& count:counts) {
                if(!(stream>>token)){valid=false;break;}
                if(token=="--")count=0;
                else if(!token.empty() && token.size()<=2 && std::all_of(token.begin(),token.end(),[](char c){return c>='0' && c<='9';}))count=std::stoi(token);
                else{valid=false;break;}
            }
            if(valid && !(stream>>token))allocatedVoices=counts;
            const int requested=host.presets().requestedVoices(active);
            put(result,"allocatedVoices",allocatedVoices[static_cast<size_t>(active)]<0?juce::var():juce::var(allocatedVoices[static_cast<size_t>(active)]));
            put(result,"requestedVoices",requested<0?juce::var():juce::var(requested));
            put(result,"lcd",juce::var(juce::Array<juce::var>{clean(mc.getLcd().line(0)),clean(mc.getLcd().line(1))}));
            for (int i=0;i<18;++i) {
                const auto k=map.read(static_cast<uint32_t>(i));if(k.assigned)host.presets().confirmParameter(k.slot,k.section,k.module,k.param,k.value); auto knob=object();
                put(knob,"index",i);put(knob,"assigned",k.assigned);put(knob,"slot",static_cast<int>(k.slot));
                put(knob,"binding",juce::String(binding(k)));put(knob,"module",juce::String(k.moduleName));
                put(knob,"parameter",juce::String(k.paramName));put(knob,"value",static_cast<int>(k.value));put(knob,"max",static_cast<int>(k.max));
                put(knob,"display",k.assigned?juce::String(g1::formatValue(k.type,k.param,k.value)):juce::String("Unassigned"));knobs.add(knob);
            }
        }
        put(result,"knobs",juce::var(knobs)); return result;
    }
    juce::var presets(bool refresh) {
        auto result=object();juce::Array<juce::var> entries;bool complete=true;
        for (int bank=0;bank<PresetsLink::Banks;++bank) {
            PresetsLink::BankNames names;
            const bool known=host.presets().bank(bank,names);complete=complete&&known;
            if (refresh) host.presets().readBank(bank);
            if (known) for (int position=0;position<PresetsLink::Positions;++position) {
                if(names[position].empty())continue;
                auto entry=object();put(entry,"bank",bank);put(entry,"position",position);put(entry,"name",juce::String(names[position]));entries.add(entry);
            }
        }
        put(result,"complete",complete);put(result,"entries",juce::var(entries));return result;
    }
    void releasePad(int pad) {
        auto& held=heldPads[static_cast<size_t>(pad)];
        if(held.channel>=0 && (host.running() || !std::is_same_v<Host,EmuHost>)) {
            const uint8_t message[]={static_cast<uint8_t>(0x80|held.channel),static_cast<uint8_t>(held.note),0};
            host.queueMidiIn(message,sizeof(message));
        }
        held={};
    }
    void releasePads() {for(int i=0;i<16;++i)releasePad(i);}
    void padMidi(const juce::var& request) {
        const int pad=integer(request,"pad",0,15),slot=integer(request,"slot",0,3),note=integer(request,"note",0,127),velocity=integer(request,"velocity",0,127);
        if(velocity==0){releasePad(pad);return;}
        if(!ready())throw juce::String("G1-Emu is not ready for MIDI pads");
        if(host.presets().patchOperationBusy())throw juce::String("Wait for patch loading before playing pads");
        SynthSettings settings;uint64_t revision=0;
        if(!host.synthSettings().settings(settings,revision))throw juce::String("MIDI settings unavailable");
        const int channel=settings.midiChannel[static_cast<size_t>(slot)];
        if(channel<0 || channel>=16)throw juce::String("The selected slot MIDI channel is Off; enable it in Settings");
        auto& held=heldPads[static_cast<size_t>(pad)];
        if(held.channel==channel && held.note==note)return;
        releasePad(pad);
        const uint8_t message[]={static_cast<uint8_t>(0x90|channel),static_cast<uint8_t>(note),static_cast<uint8_t>(velocity)};
        host.queueMidiIn(message,sizeof(message));held={channel,note};
    }
    juce::var handle(const juce::var& request) {
        if (!request.isObject() || !request.getProperty("method",{}).isString()) throw juce::String("Invalid request");
        const auto method=request["method"].toString();
        if(method=="status")return status();
        if(method=="pad_panic" || method=="pad_midi") {
            if(method=="pad_panic")releasePads();else padMidi(request);
            auto result=object();put(result,"accepted",true);return result;
        }
        if(!ready())throw juce::String("G1-Emu is not ready; wait for OS boot and configure a compatible local ROM");
        if(host.presets().patchOperationBusy() && (method=="set_knobs"||method=="load_file"||method=="load_patch"||method=="select_slot"))throw juce::String("Wait for the current patch operation to finish");
        if(method=="presets") {
            const auto refresh=request.getProperty("refresh",false);
            if(!refresh.isBool())throw juce::String("refresh must be boolean");
            return presets(static_cast<bool>(refresh));
        }
        if(method=="set_control")setControl(request);
        else if(method=="function") {
            const auto pressed=request.getProperty("pressed",{});if(!pressed.isBool())throw juce::String("pressed must be boolean");
            functionHeld=static_cast<bool>(pressed);host.mc().setButton(2,7,functionHeld);
        } else if(method=="show_settings") {
            if constexpr(!std::is_same_v<Host,EmuHost>){if(!host.canShowSettings())throw juce::String("Open this plugin's editor in the DAW to show Settings");}
            if(!onShowSettings)throw juce::String("Open G1-Emu's editor to show native Settings");
            auto life=lifetime;auto callback=onShowSettings.get();
            juce::MessageManager::callAsync([life,callback]{{std::lock_guard<std::mutex> lock(life->gate);if(!life->alive)return;}callback();});
        } else if(method=="set_knobs") {
            auto targets=request.getProperty("targets",{});auto* array=targets.getArray();
            if(!array || array->size()>18)throw juce::String("targets must contain at most 18 knobs");
            g1::KnobMap map(host.mc());std::vector<std::pair<g1::KnobInfo,uint8_t>> writes;std::vector<int> indices;
            // Validate the entire batch before writing anything. Stale assignments are rejected.
            for(const auto& target:*array) {
                const int index=integer(target,"index",0,17);const auto k=map.read(static_cast<uint32_t>(index));
                if(!k.assigned || target.getProperty("binding",{}).toString()!=juce::String(binding(k)))throw juce::String("Stale or unassigned knob; refresh state");
                const int value=integer(target,"value",0,k.max);
                writes.emplace_back(k,static_cast<uint8_t>(value));indices.push_back(index);
            }
            if(!functionHeld)for(size_t i=0;i<writes.size();++i) {
                const auto& k=writes[i].first;const auto value=writes[i].second;
                if(!host.presets().setParameter(k.slot,k.section,k.module,k.param,value))
                    host.mc().setAdc(g1::KnobMap::KnobAdc[static_cast<size_t>(indices[i])],g1::KnobMap::positionFor(value,k.max));
                if constexpr(!std::is_same_v<Host,EmuHost>)host.mirrorControllerKnob(indices[i],value,k.max);
            }
        } else if(method=="select_slot") {
            const int slot=integer(request,"slot",0,3);
            // Match an ordinary panel press/release, not a write into the OS's state RAM.
            if(releaseBit>=0)host.mc().setButton(0,static_cast<uint32_t>(releaseBit),false);
            releaseBit=slot+2;host.mc().setButton(0,static_cast<uint32_t>(releaseBit),true);
            releaseAt=juce::Time::getMillisecondCounterHiRes()+60.0;
        } else if(method=="load_file") {
            const int slot=integer(request,"slot",0,3);const auto path=request.getProperty("path",{});
            if(!path.isString() || !juce::File::isAbsolutePath(path.toString()))throw juce::String("Expected absolute local .pch path");
            const juce::File file(path.toString());
            if(!file.hasFileExtension("pch") || !file.existsAsFile() || file.getSize()>1024*1024)throw juce::String("Expected existing .pch file of at most 1 MiB");
            auto upload=g1gui::preparePch(file,slot);if(!upload.ok())throw upload.error;
            host.presets().upload(slot,std::move(upload.frames),std::move(upload.abort),-1,0);
        } else if(method=="load_patch") {
            const int slot=integer(request,"slot",0,3),bank=integer(request,"bank",0,8),position=integer(request,"position",0,98);
            host.presets().load(slot,bank,position);
        } else throw juce::String("Unknown method");
        if constexpr(!std::is_same_v<Host,EmuHost>)host.controllerChanged(request);
        auto result=object();put(result,"accepted",true);return result;
    }
    void queue(const juce::var& value) {
        output+=juce::JSON::toString(value,true).toStdString()+"\n";
        if(output.size()>1024*1024)closeClient();
    }
    void closeClient(){
        std::lock_guard<std::mutex> lock(lifetime->gate);++lifetime->generation;
        releasePads();
        // Standalone buttons are atomic; plugin cleanup uses its lifecycle guard.
        if constexpr(std::is_same_v<Host,EmuHost>) {
            if(host.running()) {
                if(functionHeld)host.mc().setButton(2,7,false);
                if(releaseBit>=0)host.mc().setButton(0,static_cast<uint32_t>(releaseBit),false);
            }
        } else host.releaseControllerButtons(functionHeld,releaseBit);
        functionHeld=false;releaseBit=-1;lifetime->heartbeat=false;client.reset();input.clear();output.clear();responses.clear();
    }
    void releaseButtons() {
        if(releaseBit>=0 && juce::Time::getMillisecondCounterHiRes()>=releaseAt) {
            if(host.running()){host.mc().setButton(0,static_cast<uint32_t>(releaseBit),false);host.synthSettings().read();}releaseBit=-1;
        }
    }
    void pollTransport() {
        const auto life=lifetime;
        if(!host.running()) {
            life->heartbeat=false;
            if constexpr(!std::is_same_v<Host,EmuHost>) {
                std::lock_guard<std::mutex> lock(life->gate);releasePads();
                host.releaseControllerButtons(functionHeld,releaseBit);functionHeld=false;releaseBit=-1;
            }
        }
        const auto now=std::chrono::steady_clock::now();
        if(now>=nextHeartbeat && !life->heartbeat.exchange(true)) {
            nextHeartbeat=now+std::chrono::milliseconds(4);
            if(!host.postMachineTask([this,life]{
                std::lock_guard<std::mutex> lock(life->gate);
                if(life->alive)releaseButtons();life->heartbeat=false;
            }))life->heartbeat=false;
        }
        if(listener.waitUntilReady(true,0)>0) {
            std::unique_ptr<juce::StreamingSocket> incoming(listener.waitForNextConnection());
            if(!client && incoming) {
                client=std::move(incoming);
#if JUCE_MAC
                int one=1;::setsockopt(client->getRawSocketHandle(),SOL_SOCKET,SO_NOSIGPIPE,&one,sizeof(one));
#endif
                auto hello=object();put(hello,"service","nord-modular-ni-interface-g1");put(hello,"protocol",1);put(hello,"instance",instance);hostInfo(hello);queue(hello);
            }
        }
        if(!client)return;
        if(client->waitUntilReady(true,0)>0) {
            char bytes[8192];const int count=client->read(bytes,sizeof(bytes),false);
            if(count<=0){closeClient();return;}input.append(bytes,static_cast<size_t>(count));
        }
        if(input.size()>65536){closeClient();return;}
        for(int count=0;count<8 && client && responses.size()<8;++count) {
            const auto newline=input.find('\n');if(newline==std::string::npos)break;
            const auto line=input.substr(0,newline);input.erase(0,newline+1);
            juce::var request;
            const auto parsed=juce::JSON::parse(juce::String::fromUTF8(line.data(),static_cast<int>(line.size())),request);
            const auto generation=life->generation;
            if(!host.running()) {
                auto response=object();put(response,"id",request.getProperty("id",{}));
                if(parsed.failed())put(response,"error","Malformed JSON");
                else if(request.getProperty("method",{}).toString()!="status")put(response,"error","G1-Emu is not running");
                else {
                    // The engine may start while this response is built. Do not read
                    // host RAM/options on this I/O thread, even in the stopped path.
                    auto result=object();put(result,"running",false);hostInfo(result);put(result,"activeSlot",0);
                    put(result,"encoderInput","editor-parameters");put(result,"engineThreadRequests",std::is_same_v<Host,EmuHost>);put(result,"transportThread","dedicated");put(result,"padMidi",true);
                    put(result,"knobs",juce::var(juce::Array<juce::var>{}));put(result,"padMidiHeld",juce::var(juce::Array<juce::var>{}));
                    put(result,"lcd",juce::var(juce::Array<juce::var>{"",""}));put(result,"configuration",object());
                    auto operation=object();put(operation,"serial",static_cast<juce::int64>(0));put(operation,"busy",false);put(operation,"ok",false);put(operation,"message","G1-Emu is stopped");
                    put(result,"upload",operation);put(result,"bankLoad",operation);put(response,"result",result);
                }
                queue(response);continue;
            }
            auto task=std::make_shared<std::packaged_task<juce::var()>>([this,life,generation,request,parsed]{
                std::lock_guard<std::mutex> lock(life->gate);
                if(!life->alive || generation!=life->generation)return juce::var();
                auto response=object();put(response,"id",request.getProperty("id",{}));
                try {
                    if(parsed.failed())throw juce::String("Malformed JSON");
                    put(response,"result",handle(request));
                }catch(const juce::String& error){put(response,"error",error);}
                catch(const std::exception& error){put(response,"error",juce::String(error.what()));}
                return response;
            });
            auto future=task->get_future();
            if(host.postMachineTask([task]{(*task)();}))responses.push_back({generation,std::move(future)});
            else {auto response=object();put(response,"id",request.getProperty("id",{}));put(response,"error","Engine request queue is busy");queue(response);}
        }
        while(!responses.empty() && responses.front().future.wait_for(std::chrono::seconds(0))==std::future_status::ready) {
            auto pending=std::move(responses.front());responses.pop_front();
            try {auto response=pending.future.get();if(pending.generation==life->generation && !response.isVoid())queue(response);}
            catch(...){std::fprintf(stderr,"MK3 bridge: engine request failed; closing client for clean retry\n");closeClient();break;}
        }
        if(!client || output.empty())return;
#if JUCE_MAC || JUCE_LINUX
        const auto count=::send(client->getRawSocketHandle(),output.data(),std::min<size_t>(output.size(),65536),MSG_DONTWAIT
#ifdef MSG_NOSIGNAL
            | MSG_NOSIGNAL
#endif
        );
        if(count>0)output.erase(0,static_cast<size_t>(count));
        else if(count==0 || (errno!=EAGAIN && errno!=EWOULDBLOCK && errno!=EINTR))closeClient();
#else
        closeClient(); // initial integration targets the Apple Silicon standalone
#endif
    }
    struct Lifetime {std::mutex gate;bool alive=true;uint64_t generation=0;std::atomic<bool> heartbeat{false};};
    struct Response {uint64_t generation;std::future<juce::var> future;};
    std::atomic<bool> stopping{false};
    std::thread transport;
    std::chrono::steady_clock::time_point nextHeartbeat{};
    std::shared_ptr<Lifetime> lifetime=std::make_shared<Lifetime>();
    std::deque<Response> responses;
    Host& host;
    juce::StreamingSocket listener;
    std::unique_ptr<juce::StreamingSocket> client;
    const juce::String instance=juce::Uuid().toString();
    std::string input,output;
    int listeningPort=0;
    int releaseBit=-1;double releaseAt=0;
    struct HeldPad {int channel=-1;int note=0;};
    std::array<HeldPad,16> heldPads{};
    bool functionHeld=false;
    std::optional<EmuHost::Options> settingsToSave;
    bool settingsSaveScheduled=false;
    std::optional<SynthSettings> pendingSettings;
    double settingsUntil=0;
    std::array<int,4> allocatedVoices{-1,-1,-1,-1};
    int voiceSlot=-1;
    double voiceRefresh=0;
};
using Mk3ControlBridge=BasicMk3ControlBridge<EmuHost>;
}
