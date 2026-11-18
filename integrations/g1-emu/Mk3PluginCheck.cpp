// Local VST3/AU construction, audio and state harness. GPL-3.0-or-later.
// Use G1_ROM and G1_DATA_DIR pointing to isolated test data. No audio device is opened.
#include <juce_audio_processors/juce_audio_processors.h>
#include <juce_gui_basics/juce_gui_basics.h>
#include <atomic>
#include <csignal>
#include <cstdio>
#include <iostream>
#include <mutex>
#include <queue>
#include <thread>
#include <unistd.h>
#include <fcntl.h>
static std::atomic<bool> stopped{false};
static void stop(int){stopped=true;}
int main(int argc,char** argv) {
    if(argc<2){std::fprintf(stderr,"usage: g1mk3plugincheck plugin.vst3 [instances=1]\n");return 2;}
    if(!std::getenv("G1_ROM") || !std::getenv("G1_DATA_DIR")){std::fprintf(stderr,"Explicit G1_ROM and isolated G1_DATA_DIR required\n");return 2;}
    std::signal(SIGTERM,stop);std::signal(SIGINT,stop);
    juce::ScopedJuceInitialiser_GUI gui;
    juce::AudioPluginFormatManager formats;
    formats.addFormat(std::make_unique<juce::VST3PluginFormat>());
    juce::OwnedArray<juce::PluginDescription> types;
    formats.getFormat(0)->findAllTypesForFile(types,argv[1]);
    if(types.isEmpty()){std::fprintf(stderr,"Plugin not found\n");return 1;}
    constexpr double rate=48000.;constexpr int block=512;
    auto load=[&] {
        juce::String error;
        auto plugin=formats.createPluginInstance(*types[0],rate,block,error);
        if(!plugin)throw std::runtime_error(error.toStdString());
        plugin->setNonRealtime(true);plugin->setRateAndBufferSizeDetails(rate,block);plugin->prepareToPlay(rate,block);
        return plugin;
    };
    std::vector<std::unique_ptr<juce::AudioPluginInstance>> plugins;
    try{for(int n=0;n<(argc>2?std::clamp(std::atoi(argv[2]),1,2):1);++n)plugins.push_back(load());}
    catch(const std::exception& e){std::fprintf(stderr,"%s\n",e.what());return 1;}
    const int stdinFlags=fcntl(STDIN_FILENO,F_GETFL,0);fcntl(STDIN_FILENO,F_SETFL,stdinFlags|O_NONBLOCK);
    std::string pendingInput;
    std::vector<bool> paused(plugins.size(),false);
    std::vector<float> peaks(plugins.size(),0.f);
    std::vector<bool> inputEnabled(plugins.size(),false);std::vector<double> inputPhase(plugins.size(),0.);
    auto started=std::chrono::steady_clock::now(),report=started;
    std::printf("HOST READY instances=%zu name=%s latency=%d\n",plugins.size(),plugins[0]->getName().toRawUTF8(),plugins[0]->getLatencySamples());std::fflush(stdout);
    while(!stopped && std::chrono::steady_clock::now()-started<std::chrono::seconds(150)) {
        juce::MessageManager::getInstance()->runDispatchLoopUntil(1);
        std::queue<std::string> input;char bytes[4096];ssize_t count=0;
        while((count=::read(STDIN_FILENO,bytes,sizeof(bytes)))>0)pendingInput.append(bytes,static_cast<size_t>(count));
        if(count==0)stopped=true;
        for(auto end=pendingInput.find('\n');end!=std::string::npos;end=pendingInput.find('\n')){input.push(pendingInput.substr(0,end));pendingInput.erase(0,end+1);}
        while(!input.empty()) {
            auto request=juce::JSON::parse(input.front());input.pop();const auto method=request["method"].toString();
            const int index=static_cast<int>(request.getProperty("index",0));
            if(index<0 || index>=static_cast<int>(plugins.size()))continue;
            auto& plugin=plugins[static_cast<size_t>(index)];
            if(method=="quit"){stopped=true;break;}
            if(method=="input")inputEnabled[static_cast<size_t>(index)]=static_cast<bool>(request.getProperty("value",false));
            if(method=="pause"){plugin->releaseResources();paused[static_cast<size_t>(index)]=true;}
            if(method=="resume"){plugin->prepareToPlay(rate,block);paused[static_cast<size_t>(index)]=false;}
            if(method=="parameters") {
                const auto& parameters=plugin->getParameters();
                std::printf("HOST PARAMETERS index=%d first=%.8f gain=%.8f\n",index,static_cast<double>(parameters[0]->getValue()),static_cast<double>(parameters[19]->getValue()));
            }
            if(method=="save"){
                juce::MemoryBlock state;plugin->getStateInformation(state);
                const juce::File file(request["path"].toString());file.replaceWithData(state.getData(),state.getSize());
                std::printf("HOST SAVED index=%d bytes=%zu\n",index,state.getSize());
            }
            if(method=="restore" || method=="recreate") {
                juce::MemoryBlock state;const juce::File file(request["path"].toString());file.loadFileAsData(state);
                if(method=="recreate"){plugin->releaseResources();plugin.reset();plugin=load();}
                plugin->setStateInformation(state.getData(),static_cast<int>(state.getSize()));
                paused[static_cast<size_t>(index)]=false;
                std::printf("HOST RESTORED index=%d recreate=%d\n",index,method=="recreate");
            }
            std::fflush(stdout);
        }
        for(size_t i=0;i<plugins.size() && !stopped;++i)if(!paused[i]) {
            auto& plugin=*plugins[i];juce::AudioBuffer<float> buffer(std::max(2,plugin.getTotalNumOutputChannels()),block);buffer.clear();juce::MidiBuffer midi;
            if(inputEnabled[i] && plugin.getTotalNumInputChannels()>0)for(int frame=0;frame<block;++frame){
                const float sample=static_cast<float>(std::sin(inputPhase[i])*.1);inputPhase[i]+=2.*3.141592653589793*220./rate;
                for(int channel=0;channel<std::min(2,plugin.getTotalNumInputChannels());++channel)buffer.setSample(channel,frame,sample);
            }
            plugin.processBlock(buffer,midi);
            for(int channel=0;channel<std::min(2,buffer.getNumChannels());++channel)peaks[i]=std::max(peaks[i],buffer.getMagnitude(channel,0,block));
        }
        if(std::chrono::steady_clock::now()-report>=std::chrono::milliseconds(250)) {
            for(size_t i=0;i<plugins.size();++i)std::printf("HOST AUDIO index=%zu peak=%.8f paused=%d\n",i,static_cast<double>(peaks[i]),paused[i]?1:0);
            std::fflush(stdout);std::fill(peaks.begin(),peaks.end(),0.f);report=std::chrono::steady_clock::now();
        }
    }
    for(auto& plugin:plugins)plugin->releaseResources();plugins.clear();
    std::puts("HOST EXIT");std::fflush(stdout);return 0;
}
