#pragma once
#include <juce_audio_devices/juce_audio_devices.h>
#include <atomic>
#include <array>
#include <iostream>

// Additional CoreMIDI subscriber only. Never forwards, consumes, or rewrites MIDI.
class MidiActivity final : public juce::MidiInputCallback {
public:
    ~MidiActivity() override { close(); }
    static bool accepts(const juce::String& name) {
        auto n=name.toLowerCase().removeCharacters(" _-");
        return n.contains("s49") && n.contains("mk2") && !n.contains("daw") && !n.contains("port2");
    }
    void refresh() {
        const auto now=juce::Time::getMillisecondCounterHiRes();
        if(now<nextScan)return;nextScan=now+2000;
        juce::MidiDeviceInfo wanted;
        for(const auto& d:juce::MidiInput::getAvailableDevices())if(accepts(d.name)){wanted=d;break;}
        if(input && wanted.identifier==identifier)return;
        close();
        if(wanted.identifier.isEmpty())return;
        input=juce::MidiInput::openDevice(wanted.identifier,this);
        if(input){identifier=wanted.identifier;source=wanted.name;input->start();std::cerr<<"Passive MIDI activity source: "<<source<<std::endl;}
    }
    void handleIncomingMidiMessage(juce::MidiInput*,const juce::MidiMessage& message) override {
        const int channel=message.getChannel()-1;if(channel<0||channel>=16)return;lastActivity.store(juce::Time::currentTimeMillis(),std::memory_order_release);
        const bool on=message.isNoteOn(),off=message.isNoteOff();
        if(on){const int note=message.getNoteNumber(),v=message.getVelocity();keys[channel][note]=uint8_t(v);sounding[channel][note]=uint8_t(v);
            velocity.store(v,std::memory_order_relaxed);lastNote.store(juce::Time::currentTimeMillis(),std::memory_order_relaxed);serial.fetch_add(1,std::memory_order_release);
        }else if(off){const int note=message.getNoteNumber();keys[channel][note]=0;if(!pedal[channel])sounding[channel][note]=0;
        }else if(message.isController()){
            const int cc=message.getControllerNumber(),v=message.getControllerValue();
            if(cc==64){pedal[channel]=v>=64;if(!pedal[channel])for(int n=0;n<128;++n)if(!keys[channel][n])sounding[channel][n]=0;}
            else if(cc==120||cc==123){keys[channel].fill(0);sounding[channel].fill(0);pedal[channel]=false;}
            else if(cc==121){pedal[channel]=false;for(int n=0;n<128;++n)if(!keys[channel][n])sounding[channel][n]=0;}
            else return;
        }else return;
        publishHeld();
    }
    juce::var snapshot() const {
        auto* o=new juce::DynamicObject();const auto count=serial.load(std::memory_order_acquire);
        o->setProperty("activityTimestampMs",lastActivity.load(std::memory_order_acquire));o->setProperty("connected",input!=nullptr);o->setProperty("source",source);
        o->setProperty("serial",juce::int64(count));o->setProperty("velocity",velocity.load(std::memory_order_relaxed));
        o->setProperty("timestampMs",juce::int64(count?lastNote.load(std::memory_order_relaxed):0));
        o->setProperty("activeNotes",activeNotes.load(std::memory_order_acquire));o->setProperty("heldVelocity",heldVelocity.load(std::memory_order_relaxed));o->setProperty("heldPeakNotes",heldPeakNotes.load(std::memory_order_relaxed));
        o->setProperty("releaseTimestampMs",releaseTimestamp.load(std::memory_order_relaxed));o->setProperty("releaseVelocity",releaseVelocity.load(std::memory_order_relaxed));o->setProperty("releaseNotes",releaseNotes.load(std::memory_order_relaxed));
        return juce::var(o);
    }
private:
    void publishHeld(){
        int count=0,maximum=0;for(const auto& channel:sounding)for(auto v:channel)if(v){++count;maximum=juce::jmax(maximum,int(v));}
        const int previous=activeNotes.load(std::memory_order_relaxed);
        if(count){if(!previous){peakVelocity=0;peakNotes=0;}peakVelocity=juce::jmax(peakVelocity,maximum);peakNotes=juce::jmax(peakNotes,count);}
        else if(previous){releaseVelocity.store(peakVelocity,std::memory_order_relaxed);releaseNotes.store(peakNotes,std::memory_order_relaxed);releaseTimestamp.store(juce::Time::currentTimeMillis(),std::memory_order_relaxed);}
        heldVelocity.store(count?peakVelocity:0,std::memory_order_relaxed);heldPeakNotes.store(count?peakNotes:0,std::memory_order_relaxed);activeNotes.store(count,std::memory_order_release);
    }
    void close(){if(input)input->stop();input.reset();identifier.clear();source.clear();keys={};sounding={};pedal={};publishHeld();}
    // MIDI callback owns these bounded tables; snapshots read only the atomic summary.
    std::array<std::array<uint8_t,128>,16> keys{},sounding{};std::array<bool,16> pedal{};int peakVelocity=0,peakNotes=0;
    std::atomic<int> activeNotes{0},heldVelocity{0},heldPeakNotes{0},releaseVelocity{0},releaseNotes{0};std::atomic<juce::int64> releaseTimestamp{0};
    std::atomic<juce::int64> lastActivity{0};
    std::atomic<uint64_t> serial{0};std::atomic<int> velocity{0};std::atomic<juce::int64> lastNote{0};
    std::unique_ptr<juce::MidiInput> input;juce::String identifier,source;double nextScan=0;
};
