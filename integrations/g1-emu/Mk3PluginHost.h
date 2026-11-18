// Modular Machine's adapter for upstream G1-Emu plugin instances. GPL-3.0-or-later.
// It never opens an audio device. DAW audio remains in Processor/Runner.
#pragma once
#include "Processor.h"
#include "Editor.h"
#include "Mk3ControlBridge.h"
#include <condition_variable>
#include <deque>

namespace g1plugin {
class Mk3PluginHost final : private juce::AsyncUpdater {
public:
    using Options=g1app::HostOptions;
    explicit Mk3PluginHost(Processor& processor):processor(processor) {
        worker=std::thread([this]{run();});
        for(int port=47420;port<47484;++port) {
            auto candidate=std::make_unique<g1app::BasicMk3ControlBridge<Mk3PluginHost>>(*this,port);
            if(candidate->listening()) {
                controllerPort.store(port);bridge=std::move(candidate);
                bridge->onShowSettings=[this]{if(auto* editor=dynamic_cast<Editor*>(this->processor.getActiveEditor()))editor->openControllerSettings();};
                break;
            }
        }
    }
    ~Mk3PluginHost() override {
        bridge.reset();cancelPendingUpdate();
        stopping.store(true);wake.notify_all();if(worker.joinable())worker.join();
    }
    int port() const {return controllerPort.load();}
    bool running() const {return available.load(std::memory_order_acquire);}
    bool postMachineTask(std::function<void()> task) {
        std::lock_guard<std::mutex> lock(queueGate);
        if(stopping.load() || !available.load() || tasks.size()>=64)return false;
        tasks.push_back(std::move(task));wake.notify_one();return true;
    }
    void queueMidiIn(const uint8_t* bytes,size_t size) {
        if(size==0 || size>48 || size%3!=0)return;
        std::lock_guard<std::mutex> lock(queueGate);
        if(midi.size()+size>4096)return;
        midi.insert(midi.end(),bytes,bytes+size);wake.notify_one();
    }
    void releaseControllerButtons(bool function,int slotBit) {
        if(!function && slotBit<0)return;
        std::lock_guard<std::mutex> lock(queueGate);
        releaseFunction=releaseFunction||function;
        if(slotBit>=0)releaseSlots|=1u<<static_cast<unsigned>(slotBit);
        wake.notify_one();
    }
    g1::Microcontroller& mc(){return processor.m_engine->mc();}
    g1app::PresetsLink& presets(){return processor.m_presets;}
    g1app::SynthSettingsLink& synthSettings(){return processor.m_synthSettings;}
    Options& options(){return optionSnapshot;}
    g1app::HostStats stats(){auto value=processor.m_runner->stats();value.midiIn+=controllerMidiBytes;return value;}
    std::string bindingEpoch() const {return std::to_string(processor.m_generation.load());}
    void setGainDb(float gain) {
        static_cast<juce::AudioProcessorParameter&>(*processor.m_gainParam).setValue(processor.m_gainParam->convertTo0to1(gain));
        processor.m_runner->setGainDb(gain);optionSnapshot.gainDb=gain;
        gainToHost.store(gain);triggerAsyncUpdate();
    }
    void mirrorControllerKnob(int index,int value,int max) {
        // Existing upstream ADC/host-gesture reconciliation reports this edit to
        // the DAW; the direct parameter frame still provides the fast engine path.
        mc().setAdc(g1::KnobMap::KnobAdc[static_cast<size_t>(index)],g1::KnobMap::positionFor(static_cast<uint8_t>(value),static_cast<uint8_t>(max)));
    }
    void controllerChanged(const juce::var& request) {
        const auto method=request["method"].toString();
        if(method=="load_file" || method=="load_patch")refreshSlots|=1u<<static_cast<unsigned>(static_cast<int>(request["slot"]));
        if(method=="set_knobs" || method=="set_control" || method=="load_file" || method=="load_patch" || method=="select_slot") {
            dirtyToHost.store(true);triggerAsyncUpdate();
        }
    }
    void bridgeInfo(juce::var& value) {
        value.getDynamicObject()->setProperty("hostType","plugin");
        value.getDynamicObject()->setProperty("engineAccess","runner-lock");
        value.getDynamicObject()->setProperty("pluginName",processor.getName());
        value.getDynamicObject()->setProperty("pluginInstance",controllerPort.load()-47419);
        value.getDynamicObject()->setProperty("pcPortInstance",processor.m_instance);
        value.getDynamicObject()->setProperty("engineGeneration",processor.m_generation.load());
        value.getDynamicObject()->setProperty("controllerPort",controllerPort.load());
    }
    void editorOpened(bool open){editorOpen.store(open);}
    bool canShowSettings() const {return editorOpen.load();}
private:
    void handleAsyncUpdate() override {
        const float gain=gainToHost.exchange(-1.f);
        if(gain>=0) {
            auto& parameter=processor.gainParameter();parameter.beginChangeGesture();
            parameter.setValueNotifyingHost(parameter.convertTo0to1(gain));parameter.endChangeGesture();
        }
        if(dirtyToHost.exchange(false))processor.stateChanged();
    }
    void run() {
        for(;;) {
            {
                std::unique_lock<std::mutex> lock(queueGate);
                wake.wait_for(lock,std::chrono::milliseconds(4),[this]{return stopping.load() || !tasks.empty() || !midi.empty() || releaseFunction || releaseSlots;});
            }
            // Never run in the host's audio callback. The existing lifecycle and
            // engine locks keep reload/state restore and controller reads coherent.
            std::unique_lock<std::mutex> lifecycle(processor.m_lifecycle,std::try_to_lock);
            if(!lifecycle.owns_lock()){if(stopping.load())break;std::this_thread::sleep_for(std::chrono::milliseconds(1));continue;}
            const bool ready=processor.m_engine && processor.m_runner;
            available.store(ready && !processor.m_restoring.load() && !processor.m_holdAll.load(),std::memory_order_release);
            if(!ready) {
                if(stopping.load())break;
                std::lock_guard<std::mutex> lock(queueGate);tasks.clear();
                if(processor.m_engine) {
                    // Preserve releases across a DAW pause; pending note-ons must
                    // never sound late when processing resumes.
                    std::vector<uint8_t> releases;
                    for(size_t i=0;i+2<midi.size();i+=3)if((midi[i]&0xf0)==0x80 || ((midi[i]&0xf0)==0x90 && midi[i+2]==0))releases.insert(releases.end(),midi.begin()+static_cast<long>(i),midi.begin()+static_cast<long>(i+3));
                    std::vector<uint8_t> routed;
                    routing.apply(processor.m_engine->mc(),g1app::MidiRouting::layout(&processor.m_synthSettings,processor.m_selectedControllers,processor.m_octaveShift),releases.data(),releases.size(),routed);
                    for(const auto byte:routed)processor.m_engine->mc().getSci().write(byte);
                    controllerMidiBytes+=releases.size();
                    if(releaseFunction)processor.m_engine->mc().setButton(2,7,false);
                    for(unsigned bit=0;bit<8;++bit)if(releaseSlots&(1u<<bit))processor.m_engine->mc().setButton(0,bit,false);
                }
                midi.clear();releaseFunction=false;releaseSlots=0;
                continue;
            }
            auto engineLock=processor.m_runner->lockEngine();
            optionSnapshot.romInUse=processor.m_romPath;optionSnapshot.rom=processor.m_romPath;
            optionSnapshot.expanded=processor.m_model==Processor::Model::Expanded;
            optionSnapshot.gainDb=processor.m_gainParam->get();optionSnapshot.directLink=processor.m_link.listening();
            std::deque<std::function<void()>> work;
            {std::lock_guard<std::mutex> lock(queueGate);work.swap(tasks);}
            for(auto& task:work)task();
            std::vector<uint8_t> input;bool function=false;uint32_t slots=0;
            {std::lock_guard<std::mutex> lock(queueGate);input.swap(midi);function=std::exchange(releaseFunction,false);slots=std::exchange(releaseSlots,0);}
            if(function)mc().setButton(2,7,false);
            for(unsigned bit=0;bit<8;++bit)if(slots&(1u<<bit))mc().setButton(0,bit,false);
            if(!input.empty()) {
                std::vector<uint8_t> routed;
                routing.apply(mc(),g1app::MidiRouting::layout(&processor.m_synthSettings,processor.m_selectedControllers,processor.m_octaveShift),input.data(),input.size(),routed);
                for(const auto byte:routed)mc().getSci().write(byte);
                controllerMidiBytes+=input.size();
            }
            if(refreshSlots && !processor.m_presets.patchOperationBusy() && processor.m_keeper) {
                for(unsigned slot=0;slot<4;++slot)if(refreshSlots&(1u<<slot))processor.m_keeper->reread(slot);
                refreshSlots=0;
            }
            if(stopping.load())break;
        }
        available.store(false);
    }
    Processor& processor;
    Options optionSnapshot;
    std::mutex queueGate;std::condition_variable wake;
    std::deque<std::function<void()>> tasks;std::vector<uint8_t> midi;
    bool releaseFunction=false;uint32_t releaseSlots=0,refreshSlots=0;
    std::atomic<bool> stopping{false},available{false},editorOpen{false},dirtyToHost{false};
    std::atomic<int> controllerPort{0};std::atomic<float> gainToHost{-1.f};
    uint64_t controllerMidiBytes=0;
    g1app::MidiRouting routing;
    std::thread worker;
    std::unique_ptr<g1app::BasicMk3ControlBridge<Mk3PluginHost>> bridge;
};
}
