#include "SequencerMeters.h"
#include "MidiActivity.h"
#include "ParameterNames.h"
#include <mutex>
#include <vector>
#include <cstring>
class SurfaceTrace final:public juce::MidiInputCallback{
public:
    struct Message{juce::int64 time;juce::MidiInput* source;std::array<unsigned char,8> bytes;int count;};
    void handleIncomingMidiMessage(juce::MidiInput* source,const juce::MidiMessage& message)override{
        if(message.isMidiClock()||message.isActiveSense())return;
        Message row{juce::Time::currentTimeMillis(),source,{},juce::jmin(8,message.getRawDataSize())};std::memcpy(row.bytes.data(),message.getRawData(),size_t(row.count));
        std::lock_guard<std::mutex> lock(mutex);if(messages.size()<128)messages.push_back(row);
    }
    void drain(){std::vector<Message> rows;{std::lock_guard<std::mutex> lock(mutex);rows.swap(messages);}for(const auto& row:rows){std::printf("MIDI ts=%lld source=%s bytes=",(long long)row.time,row.source->getName().toRawUTF8());for(int i=0;i<row.count;++i)std::printf("%02x ",row.bytes[size_t(i)]);std::printf("\n");}std::fflush(stdout);}
private:std::mutex mutex;std::vector<Message> messages;
};
int main(int argc,char** argv){
    if(argc>1&&(std::strcmp(argv[1],"--trace")==0||std::strcmp(argv[1],"--trace-daw")==0||std::strcmp(argv[1],"--trace-all")==0)){
        juce::ScopedJuceInitialiser_GUI init;SurfaceTrace trace;std::vector<std::unique_ptr<juce::MidiInput>> inputs;
        for(const auto& device:juce::MidiInput::getAvailableDevices())if(device.name.containsIgnoreCase("KOMPLETE KONTROL S49")||(std::strcmp(argv[1],"--trace")==0?false:device.name.containsIgnoreCase("Komplete Kontrol DAW"))){
            auto input=juce::MidiInput::openDevice(device.identifier,&trace);if(input){std::printf("LISTENING %s\n",device.name.toRawUTF8());input->start();inputs.push_back(std::move(input));}
        }
        std::printf("TRACE READY ts=%lld\n",(long long)juce::Time::currentTimeMillis());std::fflush(stdout);const auto end=juce::Time::getMillisecondCounterHiRes()+1000.0*(argc>2?juce::jlimit(1,600,std::atoi(argv[2])):60);
        while(juce::Time::getMillisecondCounterHiRes()<end){trace.drain();juce::Thread::sleep(10);}for(auto& input:inputs)input->stop();trace.drain();return 0;
    }
    if(argc>1){for(const auto& d:juce::MidiInput::getAvailableDevices())std::cout<<d.name<<"\n";return 0;}
    if(!MidiActivity::accepts("KOMPLETE KONTROL S49 MK2 Port 1") || MidiActivity::accepts("KOMPLETE KONTROL S49 MK2 Port 2") || MidiActivity::accepts("Komplete Kontrol DAW - 1") || MidiActivity::accepts("S49 MK3"))return 1;
    SequencerMeters meters;int b[5]={0,4,16,9,12},a[5]={99,98,97,96,95};
    if(meters.step(0,1,0,1000))return 30;
    meters.ingest(0,1,5,b,a,5,1000);
    if(meters.step(0,1,10,1001)!=0||meters.step(0,1,12,1001)!=4||meters.step(0,1,14,1001)||meters.step(0,1,0,1001))return 31;
    if(meters.step(1,1,10,1001)||meters.step(0,2,10,1001)||meters.step(0,1,10,3001))return 32;
    meters.ingest(1,2,0,b,a,1,1100);if(meters.step(0,1,10,1100)||meters.step(1,2,0,1100)!=0)return 33;
    meters.reset();if(meters.step(1,2,0,1100))return 34;
    ParameterNames names;auto metadata=juce::JSON::parse(R"({"version":1,"romSha256":"d9b199f27f573fdf7cd985fbd33cb9e60893a08912ab5d1bb562c77c663c997e","names":{"7:0":{"name":"Freq coars","max":127},"7:2":{"name":"KBT","max":127}}})");names.load(metadata);
    if(names.name(7,0,127,"fallback")!="Freq coars" || names.name(7,2,127,"fallback")!="KBT" || names.name(7,0,126,"fallback")!="fallback" || names.name(8,0,127,"fallback")!="fallback")return 6;
    MidiActivity activity;auto first=activity.snapshot();if(int(first["serial"])!=0 || bool(first["connected"]))return 2;
    activity.handleIncomingMidiMessage(nullptr,juce::MidiMessage::midiClock());
    if(juce::int64(activity.snapshot()["activityTimestampMs"])!=0)return 20;
    activity.handleIncomingMidiMessage(nullptr,juce::MidiMessage::pitchWheel(1,8192));
    if(juce::int64(activity.snapshot()["activityTimestampMs"])<=0 || int(activity.snapshot()["serial"])!=0)return 21;
    activity.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOn(1,60,juce::uint8(99)));
    auto note=activity.snapshot();if(int(note["serial"])!=1 || int(note["velocity"])!=99 || juce::int64(note["timestampMs"])<=0)return 3;
    activity.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOff(1,60));
    activity.handleIncomingMidiMessage(nullptr,juce::MidiMessage::midiClock());
    activity.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOn(2,60,juce::uint8(0)));
    if(int(activity.snapshot()["serial"])!=1)return 4;
    for(int i=0;i<1000;++i)activity.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOn(16,64,juce::uint8(127)));
    if(int(activity.snapshot()["serial"])!=1001)return 5;
    MidiActivity held;
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOn(1,60,juce::uint8(100)));
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOn(2,64,juce::uint8(70)));
    if(int(held.snapshot()["activeNotes"])!=2 || int(held.snapshot()["heldVelocity"])!=100)return 7;
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOff(1,60));
    if(int(held.snapshot()["activeNotes"])!=1 || int(held.snapshot()["heldVelocity"])!=100 || int(held.snapshot()["heldPeakNotes"])!=2)return 8;
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOn(2,64,juce::uint8(0)));
    if(int(held.snapshot()["activeNotes"])!=0 || int(held.snapshot()["releaseVelocity"])!=100 || juce::int64(held.snapshot()["releaseTimestampMs"])<=0)return 9;
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOn(1,60,juce::uint8(80)));
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::controllerEvent(1,64,127));
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOff(1,60));
    if(int(held.snapshot()["activeNotes"])!=1)return 10;
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::controllerEvent(1,64,0));
    if(int(held.snapshot()["activeNotes"])!=0)return 11;
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::noteOn(1,60,juce::uint8(80)));
    held.handleIncomingMidiMessage(nullptr,juce::MidiMessage::controllerEvent(1,120,0));
    if(int(held.snapshot()["activeNotes"])!=0)return 12;
    std::cout<<"PASS held/polyphonic/channel note-off, sustain/panic, labels; passive source filtering, note activity/velocity, stable idle, non-note rejection and bounded burst coalescing\n";
}
